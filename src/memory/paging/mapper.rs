// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use super::{VirtualAddress, PhysicalAddress, Page, ENTRY_COUNT};
use super::entry::*;
use super::table::{self, Table, Level4};
use memory::{PAGE_SIZE, Frame, FrameAllocate, FrameDeallocate};
use core::ptr::Unique;

/// An interface to the active page table.
pub struct Mapper {
    /// This must be `Table::P4`
    p4: Unique<Table<Level4>>,
}

impl Mapper {
    /// Returns a new Mapper.
    ///
    /// # Safety
    /// The page table must be recursively mapped, if it is not any methods using
    /// the Mapper will most likely produce undefined behaviour.
    pub const unsafe fn new() -> Mapper {
        Mapper { p4: Unique::new_unchecked(table::P4) }
    }

    /// Returns a reference to the level 4 page table.
    pub fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    /// Returns a mutable reference to the level 4 page table.
    pub fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    /// Translates a virtual to the corresponding physical
    /// address. Returns `None` if the address is not mapped
    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address))
            .map(|frame| frame.0 * PAGE_SIZE + offset)
    }

    /// Translates a virtual page to the corresponding physical frame. Returns
    /// `None` if the address is not mapped.
    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        use super::entry::HUGE_PAGE;

        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // 1GiB page?
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(HUGE_PAGE) {
                        // address must be 1GiB aligned
                        assert!(start_frame.0 % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame(start_frame.0 + page.p2_index() * ENTRY_COUNT +
                                          page.p1_index()));
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // 2MiB page?
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(HUGE_PAGE) {
                            // address must be 2MiB aligned
                            assert!(start_frame.0 % ENTRY_COUNT == 0);
                            return Some(Frame(start_frame.0 + page.p1_index()));
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    /// Maps the page to the frame with the provided flags
    /// The `PRESENT` flag is set by default. Needs an allocator as it might
    /// need to create new page tables
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocate
    {
        let mut p3 = self.p4_mut().next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | PRESENT);
    }

    /// Maps the page to some free frame with the provided flags.
    /// The free frame is allocated with the given allocator
    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocate
    {
        let frame = allocator.allocate_frame()
            .expect("Out of Memory :(");
        self.map_to(page, frame, flags, allocator)
    }

    /// Identity map the given frame with the provided Flags.
    /// The allocator is used to create a new page table if needed.
    pub fn identity_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocate
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    /// Unmaps the given page and adds all freed frames to the
    /// given allocator
    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A)
        where A: FrameDeallocate
    {
        assert!(self.translate(page.start_address()).is_some());

        let p1 = self.p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("Mapping code does not support huge pages");

        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();

        // Even after we update this value in memory,
        // it is still cached in the TLB in the CPU.
        use x86_64::VirtualAddress;
        use x86_64::instructions::tlb;
        tlb::flush(VirtualAddress(page.start_address()));

        // TODO Free p(1,2,3) table if empty
        allocator.deallocate_frame(frame);
    }
}
