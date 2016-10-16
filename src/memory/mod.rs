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
use spin::Mutex;

pub use self::area_frame_iter::AreaFrameIter;
use self::stack_frame_allocator::StackFrameAllocator;
use self::paging::PhysicalAddress;


mod area_frame_iter;
mod stack_frame_allocator;
mod paging;

pub const KERNEL_BASE: usize = 0xFFFF_FFFF_8000_0000;
pub const PAGE_SIZE: usize = 4096;

const HEAP_START: usize = 0o000_001_000_0000;
const HEAP_SIZE: usize = 100 * 1024;

lazy_static! {
    static ref ACTIVE_TABLE: Mutex<paging::ActivePageTable> = {
        unsafe {
            Mutex::new( paging::ActivePageTable::new() )
        }
    };
}

pub fn init(boot_info: &BootInformation) {
    // For this function to be safe, it must only be called once.
    assert_has_not_been_called!("memory::init must only be called once!");

    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("ELF sections tag required");

    let kernel_start = elf_sections_tag.sections()
        .filter(|s| s.is_allocated()).filter(|s| s.start_address() >= KERNEL_BASE)
        .map(|s| s.start_address() - KERNEL_BASE).min().unwrap();
    let kernel_end = elf_sections_tag.sections()
        .filter(|s| s.is_allocated()).filter(|s| s.start_address() >= KERNEL_BASE)
        .map(|s| s.end_address() - KERNEL_BASE).max().unwrap();

    println!("Physical kernel start:    {:#x}, Physical kernel end:    {:#x}",
             kernel_start,
             kernel_end);
    println!("Physical multiboot start: {:#x}, Physical multiboot end: {:#x}",
             boot_info.start_address() - KERNEL_BASE,
             boot_info.end_address() - KERNEL_BASE);

    // TODO Make a static active table
    let mut active_table = unsafe { paging::ActivePageTable::new() };

    let mut frame_allocator = unsafe {
        StackFrameAllocator::new(AreaFrameIter::new(kernel_start as usize,
                                                    kernel_end as usize,
                                                    boot_info.start_address() - KERNEL_BASE,
                                                    boot_info.end_address() - KERNEL_BASE,
                                                    memory_map_tag.memory_areas()))
    };

    paging::remap_the_kernel(&mut active_table, &mut frame_allocator, boot_info);

    use self::paging::Page;
    use hole_list_allocator;

    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(page, paging::WRITABLE, &mut frame_allocator);
    }

    unsafe {
        hole_list_allocator::init(HEAP_START, HEAP_SIZE);
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
