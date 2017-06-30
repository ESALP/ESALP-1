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

pub use self::area_frame_iter::AreaFrameIter;
use self::stack_frame_allocator::StackFrameAllocator;
use self::paging::PhysicalAddress;


/// Iterator acrossed physical frames.
mod area_frame_iter;
/// Physical frame allocator that uses a stack.
mod stack_frame_allocator;
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

/// A static `StackFrameAllocator`. Will always be Some(StackFrameAlloator)
/// after init runs. This cannot be used while `ACTIVE_TABLE` is being used,
/// or it will lock the thread.
pub static FRAME_ALLOCATOR: Mutex<Option<StackFrameAllocator>> = Mutex::new(None);

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

    unsafe { *FRAME_ALLOCATOR.lock() = Some(
        StackFrameAllocator::new(AreaFrameIter::new(kernel_start as usize,
                                                    kernel_end as usize,
                                                    boot_info.start_address() - KERNEL_BASE,
                                                    boot_info.end_address() - KERNEL_BASE,
                                                    memory_map_tag.memory_areas()))
    )}

    paging::remap_the_kernel(boot_info);

    use self::paging::Page;
    use hole_list_allocator;

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

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
pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);

    fn transfer_frames<A>(&mut self, other: &mut A)
        where A: FrameAllocator
    {
        while let Some(frame) = other.allocate_frame() {
            self.deallocate_frame(frame);
        }
    }
}
