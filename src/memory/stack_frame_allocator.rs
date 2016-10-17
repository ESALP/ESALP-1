// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use core::ptr::Unique;

use memory::{self, Frame, FrameAllocator};
use memory::paging::Page;
use memory::area_frame_iter::AreaFrameIter;
use memory::ACTIVE_TABLE;

pub struct StackFrameAllocator {
    frame_iter: AreaFrameIter,
    stack_base: Unique<Frame>,
    offset: isize,
}

impl StackFrameAllocator {
    pub unsafe fn new(area_frame_iter: AreaFrameIter) -> StackFrameAllocator {
        // The stack grows upward from the kernel page to the top of memory
        let mut allocator = StackFrameAllocator {
            frame_iter: area_frame_iter,
            stack_base: Unique::new(0o177777_777_777_000_000_0000 as *mut Frame),
            offset: 0,
        };
        let mut active_table = ACTIVE_TABLE.lock();

        active_table.map_to(Page::containing_address(allocator.stack_base.get() as *const _ as usize),
                            allocator.frame_iter.next().unwrap(),
                            memory::paging::WRITABLE,
                            &mut allocator);

        allocator
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        if self.offset % 512 == 0 {
            // If we have no more frames on the current page, attempt
            // to return the frame that is used for the stack
            if let Some(frame) = ACTIVE_TABLE.lock().translate_page(
                Page::containing_address(unsafe {
                    self.stack_base.offset(self.offset) as *const _ as usize
                }))
            {
                return Some(frame)
            }
            else if self.offset == 0 {
                // This means that there are no more frames on the stack.
                // Therefore we must try to get more from the iterator. If
                // this returns `None` there are no more free frames in the
                // system and it is defined behaviour
                return self.frame_iter.next()
            }
        }
        // This means that there are frames on the stack and it is
        // not using any pages it shouldn't. Just pop one off and
        // return it.
        let frame = unsafe {
            ::core::ptr::read(self.stack_base.offset(self.offset - 1))
        };
        self.offset -= 1;

        Some(frame)
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        if self.offset % 512 == 0 && None == ACTIVE_TABLE.lock().translate_page(
            Page::containing_address(unsafe {
                self.stack_base.offset(self.offset) as *const _ as usize
            }))
        {
            // If we're on a page boundry, make sure that the page is mapped. If
            // it is not then map it with the frame we are given.
            ACTIVE_TABLE.lock()
                .map_to(Page::containing_address(unsafe {
                            self.stack_base.offset(self.offset) as *const _ as usize
                        }),
                        frame,
                        memory::paging::WRITABLE,
                        &mut *self);
        } else {

            unsafe {
                *self.stack_base.offset(self.offset) = frame;
            }
            self.offset += 1;
        }
    }
}
