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

pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator,
}

impl TemporaryPage {
    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage
        where A: FrameAllocator
    {
        TemporaryPage {
            page: page,
            allocator: TinyAllocator::new(allocator),
        }
    }

    /// Maps the temporary page to the given frame in the active page table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable)
        -> VirtualAddress
    {
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
    pub fn map_table_frame(&mut self, frame: Frame, active_table: &mut ActivePageTable)
        -> &mut Table<Level1>
    {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }
}

struct TinyAllocator([Option<Frame>; 3]);

impl TinyAllocator {
    fn new<A>(allocator: &mut A) -> TinyAllocator
        where A: FrameAllocator
    {
        let mut f = || allocator.allocate_frame();
        TinyAllocator([f(), f(), f()])
    }
}

impl FrameAllocator for TinyAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }

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
