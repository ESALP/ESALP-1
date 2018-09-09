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
use memory::vmm::{Region, VMM, Protection};

use rlibc;
use memory::PAGE_SIZE;

type BitmapEntry = usize;
const EMPTY_ENTRY: BitmapEntry = 0;
pub const BITMAP_BASE: usize = 0o177777_777_777_000_000_0000;

fn first_bit(entry: BitmapEntry) -> u32 {
    return entry.trailing_zeros()
}

fn bitmap_place(frame: &Frame) -> (usize, BitmapEntry) {
    let offset = frame.0 / (size_of::<BitmapEntry>() * 8);
    let bit = frame.0 % (size_of::<BitmapEntry>() * 8);
    let entry = EMPTY_ENTRY | (1 << bit);
    (offset, entry)
}

fn get_frame(offset: usize, entry: &mut BitmapEntry) -> Frame {
    let first_bit = first_bit(*entry) as usize;
    // Remove frame from entry
    *entry = *entry & (!(1 << first_bit));

    Frame((offset * (size_of::<FrameBitmap>() * 8)) + first_bit)
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
    pub fn new<FA>(mut allocator: FA, page_table: &mut ActivePageTable, vmm: &mut VMM) -> FrameBitmap
        where FA: FrameAllocate
    {
        // Set bitmap start to 0o177777_777_777_000_000_0000, right above the
        // kernel.
        let mut bitmap = FrameBitmap {
            bottom: unsafe {
                Unique::new_unchecked(BITMAP_BASE as *mut BitmapEntry)
            },
            size: 0,
            current: 0,
        };
        let bitmap_addr = bitmap.bottom.as_ptr() as VirtualAddress;

        let mut curr_page = Page::containing_address(bitmap_addr);

        // Map and zero the page
        page_table.map(curr_page,
                       paging::EntryFlags::WRITABLE,
                       &mut allocator);
        unsafe {
            rlibc::memset(curr_page.start_address() as *mut u8,
                          0,
                          PAGE_SIZE);
        }

        while let Some(frame) = allocator.allocate_frame() {

            let (offset, entry) = bitmap_place(&frame);
            let addr = unsafe {
                bitmap.bottom.as_ptr().offset(offset as isize)
            };

            if offset >= bitmap.size {
                bitmap.size = offset+1;
                let p = Page::containing_address(addr as usize);
                if p != curr_page {
                    curr_page = p;
                    // Map and zero the page
                    page_table.map(curr_page,
                                   paging::EntryFlags::WRITABLE,
                                   &mut allocator);
                    unsafe {
                        rlibc::memset(curr_page.start_address() as *mut u8,
                                      0,
                                      PAGE_SIZE);
                    }
                }
            }

            unsafe {
                *addr |= entry;
            }
        }
        // Now update the VMM
        let end_address = unsafe {
            bitmap.bottom.as_ptr().offset(bitmap.size as isize) as usize
        };
        let frame_bitmap_region = Region::new("Bitmap", bitmap.bottom.as_ptr() as usize,
            end_address, Protection::WRITABLE);
        assert!(vmm.insert(frame_bitmap_region));

        bitmap
    }
}

impl FrameAllocate for FrameBitmap {
    fn allocate_frame(&mut self) -> Option<Frame> {
        let old_current = self.current;
        loop {
            // FIXME this is terrible, rewrite
            let entry = unsafe {
                    &mut*self.bottom.as_ptr().offset(self.current as isize)
            };
            if *entry != 0 {
                let f = get_frame(self.current, entry);
                return Some(f);
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
