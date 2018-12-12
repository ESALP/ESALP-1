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

use core::mem::MaybeUninit;
use alloc::collections::linked_list::LinkedList;

use memory::ArchSpecificVMM;
use memory::{arch_vmm_init_preheap, arch_vmm_init};
use memory::{arch_map_to, arch_map, arch_unmap};
use memory::arch_alloc_stack;
use memory::{HEAP_START, HEAP_SIZE};
use memory::Stack;

// TODO export from arch
type vaddr = usize;
type paddr = usize;

// Entire higher half
const KERNEL_SPACE_START: vaddr = 0xffff_8000_0000_0000;
const KERNEL_SPACE_END: vaddr = 0xffff_ffff_ffff_ffff;

static KERNEL_VMM: Mutex<MaybeUninit<VMM>> = Mutex::new(MaybeUninit::uninitialized());

extern {
    static __code_start: vaddr;
    static __code_end: vaddr;
    static __bss_start: vaddr;
    static __bss_end: vaddr;
    static __data_start: vaddr;
    static __data_end: vaddr;
    static __rodata_start: vaddr;
    static __rodata_end: vaddr;
}

/// Get the real value of a symbol
macro_rules! symbol_val {
    ($sym:expr) => {{
        (&$sym as *const _ as usize)
    }}
}

// Perhaps move to arch
pub(super) fn early_regions() -> [Region; 6] {
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

/// Initialize virtual memory
pub fn vm_init(boot_info: &BootInformation) {
    assert_has_not_been_called!("vmm::vm_init must only be called once!");

    let arch_specific = arch_vmm_init_preheap(boot_info, &early_regions());
    // heap works at this point
    let mut vmm = VMM {
        start: KERNEL_SPACE_START,
        regions: LinkedList::new(),
        arch_specific: arch_specific,
        end: KERNEL_SPACE_END,
    };
    //add basic regions
    for region in early_regions().iter() {
        println!("region: {:x?}", region);
        assert!(vmm.insert(*region));
    }
    //add arch specific regions
    arch_vmm_init(&mut vmm);

    KERNEL_VMM.lock().set(vmm);
}

pub enum VmmError {
    MemUsed,
    PhysMemUsed,
    OOM
}

/// Map `region` to the paddr `start_address` or return an error
pub fn map_to(region: Region, start_address: paddr) -> Result<(),VmmError> {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };
    if !vmm.insert(region) {
        return Err(VmmError::MemUsed);
    }
    if let Err(E) = arch_map_to(&mut vmm.arch_specific, region, start_address) {
        vmm.remove_region(region.start);
        return Err(E)
    }
    Ok(())
}

/// Map `region` or return an error
pub fn map(region: Region) -> Result<(),VmmError> {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };

    if !vmm.insert(region) {
        return Err(VmmError::MemUsed);
    }
    if let Err(E) = arch_map(&mut vmm.arch_specific, region) {
        vmm.remove_region(region.start);
        return Err(E)
    }
    Ok(())
}

/// Unmap the region associated with `addr`
/// Returns `true` iff a region was unmapped
// TODO make it posssible to unmap a region
pub fn unmap(addr: vaddr) -> bool {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };

    if let Some(region) = vmm.remove_region(addr) {
        arch_unmap(&mut vmm.arch_specific, region);
        true
    } else {
        false
    }
}

/// Allocates a stack of `size` pages
// TODO fix stacks
pub fn alloc_stack(size: usize) -> Result<Stack, &'static str> {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };

    // TODO rewrite and remove arch specific
    arch_alloc_stack(&mut vmm.arch_specific, size)
}

pub struct VMM {
    start: vaddr,
    regions: LinkedList<Region>,
    //table: InactivePageTable,
    // TODO make pub(arch mem)
    pub(super) arch_specific: ArchSpecificVMM,
    end: vaddr,
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
            } else {
                break;
            }
            iter.next();
        }
        iter.insert_next(region);
        return true;
    }

   /// Returns the region that contains `address`, if it exits
   pub fn containing_region(&self, address: vaddr) -> Option<Region> {
       self.regions.iter().filter(|region| region.contains(address))
           // should contain /at most/ one region
           .next().cloned()
   }

   /// Remove the region intersecting with `address`
   pub fn remove_region(&mut self, address: usize) -> Option<Region>
   {
       self.regions.drain_filter(|region| region.contains(address))
           // should contain /at most/ one region
           .next()
   }
}

#[derive(Clone,Copy)]
#[derive(Debug)]
pub struct Region {
    name: &'static str,
    pub start: vaddr,
    pub end: vaddr,
    pub(super) protection: Protection,
}

bitflags! {
    /// Flags that are used in the entry.
    pub struct Protection: usize {
        const NONE            = 0;
        const WRITABLE        = 1 << 0;
        const USER_ACCESSIBLE = 1 << 1;
        const EXECUTABLE      = 1 << 2;
        // TODO COW
    }
}

#[derive(PartialEq)]
enum RegionOrder {
    Less,
    Greater,
    Intersects,
}

impl Region {
    pub fn new(name: &'static str, start: vaddr, end: vaddr, protection: Protection) -> Region {
        Region {
            name: name,
            start: start,
            end: end,
            protection: protection,
        }
    }

    fn contains(&self, addr: vaddr) -> bool {
        addr >= self.start && addr <= self.end
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

    // TODO unmap
    //fn difference(self, other: &Self) -> Option<(Region,Option<Region>)> {
    //}
}
