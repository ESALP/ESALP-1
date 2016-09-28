// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code,unused_variables)]

use multiboot2::BootInformation;

pub use self::area_frame_allocator::AreaFrameAllocator;
use self::paging::PhysicalAddress;


mod area_frame_allocator;
mod paging;

pub const PAGE_SIZE: usize = 4096;

pub fn init(boot_info: &BootInformation) {
    // For this function to be safe, it must only be called once.
    assert_has_not_been_called!("memory::init must only be called once!");

    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("ELF sections tag required");

    let kernel_start = elf_sections_tag.sections()
        .filter(|s| s.is_allocated()).map(|s| s.addr).min().unwrap();
    let kernel_end = elf_sections_tag.sections()
        .filter(|s| s.is_allocated()).map(|s| s.addr + s.size).max().unwrap();

    println!("Kernel start:    {:#x}, Kernel end:    {:#x}",
             kernel_start,
             kernel_end);
    println!("Multiboot start: {:#x}, Multiboot end: {:#x}",
             boot_info.start_address(),
             boot_info.end_address());

    let mut frame_allocator =  AreaFrameAllocator::new(kernel_start as usize,
                                                       kernel_end as usize,
                                                       boot_info.start_address(),
                                                       boot_info.end_address(),
                                                       memory_map_tag.memory_areas());

    let mut active_table =
        paging::remap_the_kernel(&mut frame_allocator, boot_info);

    use self::paging::Page;
    use hole_list_allocator::{HEAP_START, HEAP_SIZE};

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(page, paging::WRITABLE, &mut frame_allocator);
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(usize);

impl Frame {
    fn containing_address(address: usize) -> Frame {
        Frame(address / PAGE_SIZE)
    }

    fn start_address(&self) -> PhysicalAddress {
        self.0 * PAGE_SIZE
    }

    fn clone(&self) -> Frame {
        Frame(self.0)
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter{
            start: start,
            end: end,
        }
    }
}

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

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}
