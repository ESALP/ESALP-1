// Copyright 2017 Calvin Lee
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use core::ptr::Unique;
use core::mem::size_of;

use memory::{Frame, FrameAllocate, FrameDeallocate};
use memory::paging::{self, Page, VirtualAddress};
use memory::paging::ActivePageTable;

use rlibc;
use memory::PAGE_SIZE;

type BitmapEntry = usize;
pub const EMPTY_ENTRY: BitmapEntry = 0;

fn first_bit(entry: BitmapEntry) -> u8 {
    for bit in 0..(size_of::<usize>() * 8) {
        if entry & (1 << bit) != 0 {
            return bit as u8;
        }
    }

    unreachable!("first_bit called with empty entry")
}

fn bitmap_place(frame: &Frame) -> (usize, BitmapEntry) {
    let offset = frame.0 / (size_of::<usize>() * 8);
    let bit = frame.0 - (offset * (size_of::<usize>() * 8));
    let entry = EMPTY_ENTRY | (1 << bit);
    (offset, entry)
}

fn get_frame(offset: usize, entry: &mut BitmapEntry) -> Frame {
    let first_bit: usize = first_bit(*entry) as usize;
    // Remove frame
    *entry = *entry & (!first_bit);

    Frame((offset * (size_of::<usize>() * 8)) | first_bit)
}

/// A bitmap allocator for physical frames
pub struct FrameBitmap {
    bottom: Unique<BitmapEntry>,
    size: usize,
    current: usize,
}

impl FrameBitmap {
    /// Create a new FrameBitmap
    ///
    /// Each frame in `allocator` is consumed to create pages or generate frames
    /// to place in the bitmap. The FrameBitmap does not allocate ever after
    /// this function completes, therefore it can be used safely in conjunction
    /// with an ActivePageTable.
    pub fn new<FA>(mut allocator: FA, page_table: &mut ActivePageTable) -> FrameBitmap
        where FA: FrameAllocate
    {
        // Set bitmap start to 0o177777_777_777_000_000_0000, right above the
        // kernel.
        let mut bitmap = FrameBitmap {
            bottom: unsafe {
                Unique::new_unchecked(0o177777_777_777_000_000_0000 as *mut BitmapEntry)
            },
            size: 0,
            current: 0,
        };
        let bitmap_addr = bitmap.bottom.as_ptr() as VirtualAddress;

        let bitmap_page = Page::containing_address(bitmap_addr);

        // Map and zero the bitmap page
        page_table.map(bitmap_page,
                       paging::WRITABLE,
                       &mut allocator);
        unsafe {
            rlibc::memset(bitmap.bottom.as_ptr() as *mut u8,
                          0,
                          PAGE_SIZE);
        }

        while let Some(frame) = allocator.allocate_frame() {
            let (offset, entry) = bitmap_place(&frame);

            if offset >= bitmap.size {
                bitmap.size = offset;
            }

            unsafe {
                *bitmap.bottom.as_ptr().offset(offset as isize) |= entry;
            }
        }
        bitmap
    }
}

impl FrameAllocate for FrameBitmap {
    fn allocate_frame(&mut self) -> Option<Frame> {
        let old_current = self.current;
        loop {
            let mut entry = unsafe {
                ::core::ptr::read(
                    self.bottom.as_ptr().offset(self.current as isize)
                )
            };
            if entry != 0 {
                return Some(get_frame(self.current, &mut entry));
            }

            self.current += 1;
            if self.current == self.size {
                self.current = 0;
            }
            if self.current == old_current {
                return None;
            }
        }
    }
}

impl FrameDeallocate for FrameBitmap {
    fn deallocate_frame(&mut self, frame: Frame) {
        let (offset, entry) = bitmap_place(&frame);
        unsafe {
            *self.bottom.as_ptr().offset(offset as isize) |= entry;
        }
    }
}
