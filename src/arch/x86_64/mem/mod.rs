// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code,unused_variables)]

// TODO FIX VISIBILITY ANNOTATIONS 

use multiboot2::BootInformation;

pub use self::stack_allocator::Stack;

use self::area_frame_allocator::AreaFrameAllocator;
use self::frame_bitmap::FrameBitmap;
use self::paging::PhysicalAddress;
use self::paging::TemporaryPage;
use self::paging::{ActivePageTable, InactivePageTable};
use self::paging::{EntryFlags, Page};
use vmm::*;

/// Allocator for stacks
mod stack_allocator;
/// Allocator for physical frames.
mod area_frame_allocator;
/// Physical frame allocator that uses a bitmap.
mod frame_bitmap;
/// Virtual paging module.
mod paging;

/// The kernel is linked to `KERNEL_BASE + 1M`
const KERNEL_BASE: usize = 0xFFFF_FFFF_8000_0000;
/// The size of a single page (or physical frame)
pub const PAGE_SIZE: usize = 4096;

// exports for the vmm
/// The beginning of the kernel address space
pub const KERNEL_SPACE_START: Vaddr = 0xffff_8000_0000_0000;
/// The end of the kernel address space
pub const KERNEL_SPACE_END: Vaddr = 0xffff_ffff_ffff_ffff;


// TODO Replace this with a dynamic heap
/// The begining of the kernel heap
const HEAP_START: usize = 0o000_001_000_0000;
/// The size of the kernel heap
const HEAP_SIZE: usize = 25 * PAGE_SIZE;

/// Get the real value of a symbol
macro_rules! symbol_val {
    ($sym:expr) => {{
        (&$sym as *const _ as usize)
    }}
}

// different constants from the linker. We use this to create the early kernel regions
extern {
    static __code_start: Vaddr;
    static __code_end: Vaddr;
    static __bss_start: Vaddr;
    static __bss_end: Vaddr;
    static __data_start: Vaddr;
    static __data_end: Vaddr;
    static __rodata_start: Vaddr;
    static __rodata_end: Vaddr;
}

// XXX no way to put this in a static?
/// Each of the kernel's early memory regions
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

/// A struct that gives access to the physical and virtual memory managers.
pub struct ArchSpecificVMM {
    active_table:ActivePageTable,
    frame_allocator: FrameBitmap,
    stack_allocator: stack_allocator::StackAllocator,
}


/// Map each region in `regions` to the higher half and return the old containing page of the `p4`
/// table.
pub fn map_regions_early<FA>(regions: &[Region], active_table: &mut ActivePageTable,
                   allocator: &mut FA, boot_info: &BootInformation) -> (Page, TemporaryPage)
        where FA: FrameAllocate
{
    // Create new inactive table using a temporary page
    let mut temporary_page =
        TemporaryPage::new(Page::containing_address(0xdeadbeef), allocator);
    let mut new_table = {
        let frame = allocator.allocate_frame()
            .expect("No more frames");
        InactivePageTable::new(frame, active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        for region in regions.iter() {
            // construct flags from region flags
            // All kernel sections are global
            let flags = EntryFlags::from_protection(region.protection);

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

fn region_range(region: Region) -> paging::PageIter
{
    Page::range_inclusive(
        Page::containing_address(region.start),
        Page::containing_address(region.end))
}

/// Initializes memory to a defined state.
///
/// It first finds, and prints out, the kernel start and finish. Then it
/// remaps the kernel using correct permissions and finally allocates a
/// space for and initializes the kernel heap
pub fn arch_vmm_init_preheap(boot_info: &BootInformation) -> ArchSpecificVMM {
    let regions = early_regions();

    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("ELF sections tag required");

    let kernel_start = elf_sections_tag.sections()
        .filter(|s| s.is_allocated())
        .filter(|s| s.start_address() >= KERNEL_BASE)
        .map(|s| s.start_address() - KERNEL_BASE)
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag.sections()
        .filter(|s| s.is_allocated())
        .filter(|s| s.start_address() >= KERNEL_BASE)
        .map(|s| s.end_address() - KERNEL_BASE)
        .max()
        .unwrap();

    println!("Physical kernel start:    {:#x}, Physical kernel end:    {:#x}",
             kernel_start,
             kernel_end);
    println!("Physical multiboot start: {:#x}, Physical multiboot end: {:#x}",
             boot_info.start_address() - KERNEL_BASE,
             boot_info.end_address() - KERNEL_BASE);

    let mut active_table = unsafe {paging::ActivePageTable::new()};

    let mut frame_allocator =
        AreaFrameAllocator::new(kernel_start as usize,
                                kernel_end as usize,
                                boot_info.start_address() - KERNEL_BASE,
                                boot_info.end_address() - KERNEL_BASE,
                                boot_info,
                                memory_map_tag.memory_areas());

    let (old_p4, tmp_page) =
        map_regions_early(&regions, &mut active_table, &mut frame_allocator, boot_info);

    unsafe {
        ::hole_list_allocator::init(HEAP_START, HEAP_SIZE);
    }

    let mut frame_bitmap = FrameBitmap::new(frame_allocator, &mut active_table);
    tmp_page.consume(&mut frame_bitmap);
    active_table.unmap(old_p4, &mut frame_bitmap);

    // begone!
    let stack_allocator = {
        let alloc_start = paging::Page::containing_address(HEAP_START+HEAP_SIZE)+1;
        let alloc_end = alloc_start + 100;
        let alloc_range = paging::Page::range_inclusive(alloc_start, alloc_end);

        stack_allocator::StackAllocator::new(alloc_range)
    };

    ArchSpecificVMM {
        active_table: active_table,
        frame_allocator: frame_bitmap,
        stack_allocator: stack_allocator,
    }
}

pub fn arch_vmm_init(vmm: &mut VMM) {
    for &region in early_regions().iter() {
        vmm.insert(region);
    }
    let region = vmm.arch_specific.frame_allocator.vm_region();
    vmm.insert(region);
}

use vmm::VmmError;
pub fn arch_map_to(arch_specific: &mut ArchSpecificVMM, region: Region, start_address: usize)
    -> Result<(),VmmError>
{
    let &mut ArchSpecificVMM {
        ref mut active_table,
        ref mut frame_allocator,
        ref mut stack_allocator,
    } = arch_specific;

    if region_range(region)
            .any(|page| active_table.is_allocated(page)) {
        return Err(VmmError::MemUsed);
    }

    let flags = EntryFlags::from_protection(region.protection);

    for page in region_range(region) {
        let frame_start = start_address + (page.start_address() - region.start);
        let frame = Frame::containing_address(frame_start);
        assert!(active_table.map_to(page, frame, flags, frame_allocator).is_ok());
    }
    Ok(())
}
pub fn arch_map(arch_specific: &mut ArchSpecificVMM, region: Region)
    -> Result<(),VmmError>
{
    let &mut ArchSpecificVMM {
        ref mut active_table,
        ref mut frame_allocator,
        ref mut stack_allocator,
    } = arch_specific;
    let flags = EntryFlags::from_protection(region.protection);
    if region_range(region)
            .any(|page| active_table.is_allocated(page)) {
        return Err(VmmError::MemUsed);
    }

    for page in region_range(region) {
        active_table.map(page, flags, frame_allocator);
    }
    Ok(())
}

// XXX perhaps add an error path?
pub fn arch_unmap(arch_specific: &mut ArchSpecificVMM, region: Region)
{
    let &mut ArchSpecificVMM {
        ref mut active_table,
        ref mut frame_allocator,
        ref mut stack_allocator,
    } = arch_specific;
    for page in region_range(region) {
        active_table.unmap(page, frame_allocator);
    }
}

// TODO remove
pub fn arch_alloc_stack(arch_specific: &mut ArchSpecificVMM, size: usize)
    -> Result<Stack, &'static str>
{
    let &mut ArchSpecificVMM {
        ref mut active_table,
        ref mut frame_allocator,
        ref mut stack_allocator,
    } = arch_specific;

    stack_allocator.alloc_stack(active_table, frame_allocator, size)
}

/// A representation of a physical frame.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(usize);

impl Frame {
    /// Returns a `Frame` containing the PhysicalAddress given.
    fn containing_address(address: PhysicalAddress) -> Frame {
        Frame(address / PAGE_SIZE)
    }

    /// Returns the first address in the `Frame`
    fn start_address(&self) -> PhysicalAddress {
        self.0 * PAGE_SIZE
    }

    /// Clones the frame. This is used instead of `derive(Clone)` because cloning
    /// `Frame` is not always safe.
    fn clone(&self) -> Frame {
        Frame(self.0)
    }

    /// Returns a `Frame` iterator from `start` to `end`.
    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter {
            start: start,
            end: end,
        }
    }
}

/// An iterator acrossed `Frame`s.
struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.0 += 1;
            return Some(frame);
        }
        None
    }
}

/// A trait for the ability to allocate and deallocate `Frame`s
pub trait FrameAllocate {
    fn allocate_frame(&mut self) -> Option<Frame>;
}
pub trait FrameDeallocate {
    fn deallocate_frame(&mut self, frame: Frame);
}

/// Tests
#[cfg(feature = "test")]
pub mod tests {
    use tap::TestGroup;

    pub fn run() {
        // run the tests
        test_memory_alloc();
        //super::paging::tests::run();
    }

    fn test_memory_alloc() {
        use alloc::boxed::Box;

        let mut tap = TestGroup::new(1);
        tap.diagnostic("Testing `Box`");
        let heap_test = Box::new(42);
        tap.assert_tap(*heap_test == 42, "Could not access Box");
    }
}
