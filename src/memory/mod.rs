// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code,unused_variables)]

use spin::Mutex;

use multiboot2::BootInformation;

pub use self::stack_allocator::Stack;

use self::area_frame_allocator::AreaFrameAllocator;
use self::frame_bitmap::FrameBitmap;
use self::paging::PhysicalAddress;
use self::paging::ActivePageTable;

/// Allocator for stacks
mod stack_allocator;
/// Allocator for physical frames.
mod area_frame_allocator;
/// Physical frame allocator that uses a bitmap.
mod frame_bitmap;
/// Virtual paging module.
mod paging;

/// The kernel is linked to `KERNEL_BASE + 1M`
pub const KERNEL_BASE: usize = 0xFFFF_FFFF_8000_0000;
/// The size of a single page (or physical frame)
pub const PAGE_SIZE: usize = 4096;

// TODO Replace this with a dynamic heap
/// The begining of the kernel heap
const HEAP_START: usize = 0o000_001_000_0000;
/// The size of the kernel heap
const HEAP_SIZE: usize = 100 * 1024;

/// A struct that gives access to the physical and virtual memory managers.
struct MemoryController {
    active_table:ActivePageTable,
    frame_allocator: FrameBitmap,
    stack_allocator: stack_allocator::StackAllocator,
}

/// A static `MemoryController`. Will always be Some(_) after init completes.
static MEMORY_CONTROLLER: Mutex<Option<MemoryController>> = Mutex::new(None);


/// Allocates a stack of `size` pages
pub fn alloc_stack(size: usize) -> Option<Stack> {
    let mut lock = MEMORY_CONTROLLER.lock();
    let &mut MemoryController {
        ref mut active_table,
        ref mut frame_allocator,
        ref mut stack_allocator,
    } = lock.as_mut().unwrap();

    stack_allocator.alloc_stack(active_table,
                                frame_allocator,
                                size)
}

/// Initializes memory to a defined state.
///
/// It first finds, and prints out, the kernel start and finish. Then it
/// remaps the kernel using correct permissions and finally allocates a
/// space for and initializes the kernel heap
pub fn init(boot_info: &BootInformation) {
    // For this function to be safe, it must only be called once.
    assert_has_not_been_called!("memory::init must only be called once!");

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

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

    let mut active_table = unsafe {paging::ActivePageTable::new()};

    let frame_allocator =
        AreaFrameAllocator::new(kernel_start as usize,
                                kernel_end as usize,
                                boot_info.start_address() - KERNEL_BASE,
                                boot_info.end_address() - KERNEL_BASE,
                                memory_map_tag.memory_areas());

    let frame_bitmap =
        paging::remap_the_kernel(&mut active_table, frame_allocator, boot_info);

    let stack_allocator = {
        let alloc_start = heap_end_page + 1;
        let alloc_end = alloc_start + 100;
        let alloc_range = Page::range_inclusive(alloc_start, alloc_end);

        stack_allocator::StackAllocator::new(alloc_range)
    };

    *MEMORY_CONTROLLER.lock() = Some(MemoryController {
        active_table: active_table,
        frame_allocator: frame_bitmap,
        stack_allocator: stack_allocator,
    });

    use self::paging::Page;
    use hole_list_allocator;

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        page.map(paging::WRITABLE);
    }

    unsafe {
        hole_list_allocator::init(HEAP_START, HEAP_SIZE);
    }
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
