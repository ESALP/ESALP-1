// Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
// file at the top-level directory of this distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code,unused_variables)]

pub use self::area_frame_allocator::AreaFrameAllocator;
pub use self::paging::test_paging;
use self::paging::PhysicalAddress;

mod area_frame_allocator;
mod paging;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(usize);

impl Frame {
    fn containing_address(address: usize) -> Frame {
        Frame(address / PAGE_SIZE)
    }

    fn start_address(&self) -> PhysicalAddress {
        self.0 * PAGE_SIZE
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}
