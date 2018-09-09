// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use multiboot2::BootInformation;
use spin::Mutex;

use alloc::collections::linked_list::LinkedList;
use memory::paging::{ActivePageTable, InactivePageTable};
use memory::paging::{EntryFlags, Page};
use memory::paging::TemporaryPage;
use super::{KERNEL_BASE, Frame, FrameAllocate};
use super::{HEAP_START, HEAP_SIZE};

// Entire higher half
const KERNEL_SPACE_START: usize = 0xffff_8000_0000_0000;
const KERNEL_SPACE_END: usize = 0xffff_ffff_ffff_ffff;

extern {
    static __code_start: usize;
    static __code_end: usize;
    static __bss_start: usize;
    static __bss_end: usize;
    static __data_start: usize;
    static __data_end: usize;
    static __rodata_start: usize;
    static __rodata_end: usize;
}
/// Get the real value of a symbol
macro_rules! symbol_val {
    ($sym:expr) => {{
        (&$sym as *const _ as usize)
    }}
}

fn early_regions() -> [Region; 6] {
    unsafe { [
        // kernel
        Region {
            name: "Code",
            start: symbol_val!(__code_start),
            end: symbol_val!(__code_end),
            protection: Protection::EXECUTABLE,
        },
        Region {
            name: "BSS",
            start: symbol_val!(__bss_start),
            end: symbol_val!(__bss_end),
            protection: Protection::WRITABLE,
        },
        Region {
            name: "Data",
            start: symbol_val!(__data_start),
            end: symbol_val!(__data_end),
            protection: Protection::WRITABLE,
        },
        Region {
            name: "RoData",
            start: symbol_val!(__rodata_start),
            end: symbol_val!(__rodata_end),
            protection: Protection::NONE,
        },
        // heap
        Region {
            name: "Heap",
            start: HEAP_START,
            end: HEAP_START+HEAP_SIZE,
            protection: Protection::WRITABLE,
        },
        // VGA buffer
        Region {
            name: "VGA",
            start: 0xb8000,
            end: 0xb8008,
            protection: Protection::WRITABLE,
        }
    ]}
}

/// Create a new `VMM`. The heap must be init at this point!
pub fn vm_init_preheap<FA>(active_table: &mut ActivePageTable, allocator: &mut FA,
            boot_info: &BootInformation) -> (Page, TemporaryPage)
        where FA: FrameAllocate
{
    assert_has_not_been_called!("vmm::vm_init_preheap must only be called once!");


    // x64 specific construction
    // TODO remove
    // Create new inactive table using a temporary page
    let mut temporary_page =
        TemporaryPage::new(Page::containing_address(0xdeadbeef), allocator);
    let mut new_table = {
        let frame = allocator.allocate_frame()
            .expect("No more frames");
        InactivePageTable::new(frame, active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        for region in early_regions().iter() {
            // construct flags from region flags
            // All kernel sections are global
            let mut flags = EntryFlags::GLOBAL;
            if !region.protection.contains(Protection::EXECUTABLE) {
                flags |= EntryFlags::NO_EXECUTE;
            } if region.protection.contains(Protection::WRITABLE) {
                flags |= EntryFlags::WRITABLE;
            }
            let diff = if region.start > KERNEL_BASE {
                KERNEL_BASE
            } else {
                0
            };

            let start_frame = Frame::containing_address(region.start - diff);
            let end_frame = Frame::containing_address((region.end - diff) - 2);

            for frame in Frame::range_inclusive(start_frame, end_frame) {
                let new_page = Page::containing_address(frame.start_address() + diff);
                mapper.map_to(new_page, frame, flags, allocator)
                    .expect("Unable to map initial kernel section");
            }
        }
        // map the multiboot info section. TODO: remove
        let multiboot_start = Frame::containing_address(boot_info.start_address() - KERNEL_BASE);
        let multiboot_end = Frame::containing_address((boot_info.end_address() - KERNEL_BASE) - 1);

        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            let new_page = Page::containing_address(frame.start_address() + KERNEL_BASE);
            // if we have already mapped this page, it must have been
            // already mapped when we mapped the elf sections.
            let _ = mapper.map_to(new_page, frame, EntryFlags::PRESENT, allocator);
        }
    });
    let old_table = active_table.switch(new_table);
    println!("New page table loaded");
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address() + KERNEL_BASE);

    (old_p4_page, temporary_page)
}

pub fn vm_init() -> VMM{
    assert_has_not_been_called!("vmm::vm_init must only be called once!");
    // heap works at this point
    let mut vmm = VMM {
        start: KERNEL_SPACE_START,
        regions: LinkedList::new(),
        end: KERNEL_SPACE_END,
    };
    println!("Entered vm late");

    for region in early_regions().iter() {
        println!("region: {:x?}", region);
        assert!(vmm.insert(*region));
    }

    println!("Exited vm late");
    vmm
}

pub struct VMM {
    start: usize,
    regions: LinkedList<Region>,
    //table: InactivePageTable,
    end: usize,
}

impl VMM {
    /// Insert `region` into the VMM. Returns `false` if it intersects with an
    /// existing region.
    ///
    /// # Safety
    /// The inserted region is not actually mapped into memory.
    pub fn insert(&mut self, region: Region) -> bool {
        let mut iter = self.regions.iter_mut();
        loop {
            if let Some(next_region) = iter.peek_next() {
                match region.relation(next_region) {
                    RegionOrder::Less => (),
                    RegionOrder::Greater => break,
                    RegionOrder::Intersects => return false,
                }
            }else {
                break;
            }
            iter.next();
        }
        iter.insert_next(region);
        return true;
    }

   /// Returns the region that contains `address`, if it exits
   pub fn containing_region(&self, address: usize) -> Option<Region> {
       for region in &self.regions {
           if region.start > address {
               return None;
            }
           if region.end > address {
               return Some(*region);
           }
       }
       return None;
   }
}

#[derive(Clone,Copy)]
#[derive(Debug)]
pub struct Region {
    name: &'static str,
    start: usize,
    end: usize,
    // protections
    protection: Protection,
}

bitflags! {
    /// Flags that are used in the entry.
    pub struct Protection: u64 {
        const NONE =            0 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const EXECUTABLE =      1 << 3;
    }
}

#[derive(PartialEq)]
enum RegionOrder {
    Less,
    Greater,
    Intersects,
}

impl Region {
    pub fn new(name: &'static str, start: usize, end: usize, protection: Protection) -> Region {
        Region {
            name: name,
            start: start,
            end: end,
            protection: protection,
        }
    }

    /// Returns true iff the regions intersect
    fn intersects(&self, other: &Self) -> bool {
        self.relation(other) == RegionOrder::Intersects
    }

    fn relation(&self, other: &Self) -> RegionOrder {
        if self.end < other.start {
            RegionOrder::Less
        } else if self.start > other.end {
            RegionOrder::Greater
        } else {
            RegionOrder::Intersects
        }
    }

}
