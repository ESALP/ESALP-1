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
use memory::area_frame_iter::AreaFrameIter;
use memory::paging::{Page, VirtualAddress};
use memory::paging::ACTIVE_TABLE;
use memory::paging::TinyAllocator;

/// An allocator for physical frames using the stack.
pub struct StackFrameAllocator {
    /// This is a `Frame` pointer to the bottom of the stack. It is always at
    /// `0o177777_777_777_000_000_0000`, which is right above the kernel table.
    stack_base: Unique<Frame>,
    /// The offset to the current head of the stack.
    offset: isize,
    /// The allocator will get frames from this field if it has no more frames.
    frame_iter: AreaFrameIter,
    /// A small allocator that can be used for holding excess frames that are needed
    /// for mapping the page table.
    temp_alloc: TinyAllocator,
}

impl StackFrameAllocator {
    /// Returns a new `StackFrameAllocator`.
    ///
    /// # Safety
    /// This function can only safely be called once. It always returns an allocator
    /// with the same base address, and one may already be initialized. Thus it is
    /// up to the caller to make sure that the stack is not yet initialized, or is in
    /// a defined state before calling
    pub const unsafe fn new(area_frame_iter: AreaFrameIter) -> StackFrameAllocator {
        // The stack grows upward from the kernel page to the top of memory
        StackFrameAllocator {
            stack_base: Unique::new(0o177777_777_777_000_000_0000 as *mut Frame),
            offset: 0,
            frame_iter: area_frame_iter,
            temp_alloc: TinyAllocator::empty(),
        }
    }
}
impl FrameAllocator for StackFrameAllocator {
    /// Allocates a frame on the stack.
    ///
    /// If the allocator crosses a page boundry it will attempt to return the `Frame`
    /// mapped to the then unused stack page. If the allocator has nothing on the
    /// stack and no pages are mapped to it, `frame_iter.next()` will be returned.
    ///
    /// # Blocking
    /// `allocate frame` may block if `ACTIVE_TABLE` is being used when it is
    /// called.
    fn allocate_frame(&mut self) -> Option<Frame> {
        if self.offset % 512 == 0 {
            // If we have no more frames on the current page, attempt
            // to return the frame that is used for the stack
            let stack_page = Page::containing_address(unsafe {
                self.stack_base.offset(self.offset) as *const _ as VirtualAddress
            });
            if let Some(frame) = ACTIVE_TABLE.lock().translate_page(stack_page)
            {
                // See if there are any frames in the temp allocator
                if let Some(frame) = self.temp_alloc.allocate_frame() {
                    // If there is one, return it. Unmapping will not work if
                    // there are any frames in the allocator.
                    return Some(frame);
                }

                // If not then unmap the stack frame.
                ACTIVE_TABLE.lock().unmap(stack_page, &mut self.temp_alloc);

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
        self.offset -= 1;
        let frame = unsafe { ::core::ptr::read(self.stack_base.offset(self.offset)) };

        Some(frame)
    }

    /// Deallocates a frame on the stack.
    ///
    /// This function pushes the frame to the stack. If it crosses a page boundry
    /// and the next page is not mapped, it will map the provided frame to that page.
    ///
    /// # Safety
    /// If the stack ever completely fills up (with 512Gb free) it will cause undefined
    /// behaviour.
    ///
    /// # Blocking
    /// `deallocate frame` _may_ block if `ACTIVE_TABLE` is being used when it is
    /// called.
    fn deallocate_frame(&mut self, frame: Frame) {
        let stack_page = Page::containing_address(unsafe {
            self.stack_base.offset(self.offset) as *const _ as VirtualAddress
        });
        if self.offset % 512 == 0 && None == ACTIVE_TABLE.lock()
            .translate_page(stack_page)
        {
            // Check to see if the tiny allocator is full.
            if !self.temp_alloc.is_full() {
                // If it's not then we can't map a new page with it. Pass the
                // deallocation to it.
                self.temp_alloc.deallocate_frame(frame);
                return;
            }
            // If it is then map the frame to the stack.
            ACTIVE_TABLE.lock().map_to(stack_page,
                                       frame,
                                       memory::paging::WRITABLE,
                                       &mut self.temp_alloc);
        } else {
            // Just push a frame on the stack.
            unsafe {
                *self.stack_base.offset(self.offset) = frame;
            }
            self.offset += 1;
        }
    }
}
