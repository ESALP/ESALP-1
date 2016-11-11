// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use super::{Page, ActivePageTable, VirtualAddress};
use super::table::{Table, Level1};
use memory::{Frame, FrameAllocator};

/// A page to temporarily map a frame.
pub struct TemporaryPage {
    /// The page itself.
    page: Page,
    /// A temporary allocator.
    allocator: TinyAllocator,
}

impl TemporaryPage {
    /// Initializes a `TinyAllocator` and returns the new `TemporaryPage`
    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage
        where A: FrameAllocator
    {
        TemporaryPage {
            page: page,
            allocator: TinyAllocator::new(|| allocator.allocate_frame()),
        }
    }

    /// Maps the temporary page to the given frame in the active page table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable) -> VirtualAddress {
        use super::entry::WRITABLE;

        assert!(active_table.translate_page(self.page).is_none(),
                "Temporary page is already mapped");

        active_table.map_to(self.page, frame, WRITABLE, &mut self.allocator);
        self.page.start_address()
    }

    /// Unmaps the temparary page in the page table
    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page, &mut self.allocator)
    }

    /// Maps the temporary page to the given page table frame in the active
    /// page table. Returns a reference to the now mapped table.
    /// Returns a Table<Level1> because it is not recursively mapped
    pub fn map_table_frame(&mut self,
                           frame: Frame,
                           active_table: &mut ActivePageTable)
                           -> &mut Table<Level1> {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }

    /// This is a hack that drops the `TemporaryPage` without leaking frames. The
    /// drop trait cannot be used because this function needs a `FrameAllocator`
    pub fn drop<A>(mut self, allocator: &mut A)
        where A: FrameAllocator
    {
        self.allocator.drop(allocator);
    }
}

/// A `FrameAllocator` that can allocate up to three frames: enough for a p3, p2 and
/// p1 table.
pub struct TinyAllocator([Option<Frame>; 3]);

impl TinyAllocator {
    /// Constructs a new TinyAllocator using the given closure.
    pub fn new<F>(mut f: F) -> TinyAllocator
        where F: FnMut() -> Option<Frame>
    {
        TinyAllocator([f(), f(), f()])
    }

    /// Constructs an empty TinyAllocator.
    pub const fn empty() -> TinyAllocator {
        TinyAllocator([None, None, None])
    }

    /// Replaces all `None` fields of the allocator with new `Frames` from the given
    /// closure.
    pub fn refill<F>(&mut self, mut f: F)
        where F: FnMut() -> Frame
    {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(f());
            }
        }
    }

    /// Flushes each frame in the allocator to the given closure.
    pub fn flush<F>(&mut self, mut f: F)
        where F: FnMut(Frame)
    {
        for frame_option in &mut self.0 {
            if let Some(ref frame) = *frame_option {
                // Cloning is safe in this context because the original is
                // immediatly destroyed after the clone.
                f(frame.clone());
            }
            *frame_option = None;
        }
    }

    /// Returns `true` if the allocator is full.
    pub fn is_full(&self) -> bool {
        for frame_option in &self.0 {
            if frame_option.is_none() {
                return false;
            }
        }
        true
    }

    /// Returns `true` if the allocator is empty.
    pub fn is_empty(&self) -> bool {
        self.0 == [None, None, None]
    }

    /// This is a hack that drops the `TinyAllocator` without leaking frames. The
    /// drop trait cannot be used because this function needs a `FrameAllocator`
    pub fn drop<A>(&mut self, allocator: &mut A)
        where A: FrameAllocator
    {
        allocator.transfer_frames(self);
    }
}

impl FrameAllocator for TinyAllocator {
    /// Allocates any one of the three frames to the caller. If all three are used
    /// it returns `None`.
    fn allocate_frame(&mut self) -> Option<Frame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }

    /// Saves the frame to any unused space in the allocator.
    ///
    /// # Panics
    /// This function panics if it is called when all frames are already full. It
    /// cannot be used to hold more than three frames.
    fn deallocate_frame(&mut self, frame: Frame) {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return;
            }
        }
        panic!("Tiny allocator can only hold 3 frames.");
    }
}
