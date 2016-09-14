// Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
// file at the top-level directory of this distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

pub use self::entry::*;
use self::table::{Table, Level4};
use memory::{PAGE_SIZE, Frame, FrameAllocator};

use core::ptr::Unique;

mod entry;
mod table;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub struct Page(usize);

impl Page {
    fn start_address(&self) -> usize {
        self.0 * PAGE_SIZE
    }

    fn containing_address(address: VirtualAddress) -> Page {
        assert!(address < 0x0000_8000_0000_0000 ||
                address >=0xffff_8000_0000_0000,
                "invalid address: 0x{:x}",address);
        Page( address / PAGE_SIZE )
    }

    fn p4_index(&self) -> usize {
        (self.0 >> 27) & 0o777
    }

    fn p3_index(&self) -> usize {
        (self.0 >> 18) & 0o777
    }

    fn p2_index(&self) -> usize {
        (self.0 >>  9) & 0o777
    }

    fn p1_index(&self) -> usize {
        (self.0 >>  0) & 0o777
    }
}

pub struct ActivePageTable {
    p4: Unique<Table<Level4>>,
}

impl ActivePageTable {
    pub unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            p4: Unique::new(table::P4),
        }
    }

    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.get() }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.get_mut() }
    }

    pub fn translate(&self, virtual_address: VirtualAddress)
        -> Option<PhysicalAddress>
    {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address))
            .map(|frame| frame.0 * PAGE_SIZE + offset)
    }

    fn translate_page(&self, page: Page) -> Option<Frame> {
        use self::entry::HUGE_PAGE;

        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // 1GiB page?
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(HUGE_PAGE) {
                        // address must be 1GiB aligned
                        assert!(start_frame.0 % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame(
                                start_frame.0 + page.p2_index() *
                                ENTRY_COUNT + page.p1_index()));
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // 2MiB page?
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(HUGE_PAGE) {
                            // address must be 2MiB alligned
                            assert!(start_frame.0 % ENTRY_COUNT == 0);
                            return Some( Frame(start_frame.0 + page.p1_index()) );
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

    pub fn map_to<A>(&mut self, page: Page, frame: Frame,
                     flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator
    {
        let mut p4 = self.p4_mut();
        let mut p3 = p4.next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | PRESENT);
    }

    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator
    {
        let frame = allocator.allocate_frame()
            .expect("Out of Memory :(");
        self.map_to(page, frame, flags, allocator)
    }

    pub fn identity_map<A>(&mut self, frame: Frame,
                           flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator
    {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, allocator)
    }

    fn unmap<A>(&mut self, page: Page, allocator: &mut A)
        where A: FrameAllocator
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
        // Use ::x86 crate to flush it.
        unsafe {
            ::x86::tlb::flush(page.start_address());
        }

        // HERE BE DRAGONS
        //
        // TODO Free p(1,2,3) table if empty
        // TODO impliment deallocate_frame
        // allocator.deallocate_frame(frame);
    }
}

pub fn test_paging<A>(allocator: &mut A)
    where A: FrameAllocator
{
    let mut page_table = unsafe { ActivePageTable::new() };

    // Address 0 is mapped
    println!("Some = {:?}", page_table.translate(0));
    // Second P1 entry
    println!("Some = {:?}", page_table.translate(4096));
    // Second P2 entry
    println!("Some = {:?}", page_table.translate(4096 * 512));
    // 300th P2 entry
    println!("Some = {:?}", page_table.translate(4096 * 512 * 300));
    // Second P3 entry
    println!("None = {:?}", page_table.translate(4096 * 512 * 512));
    // Last entry
    println!("Some = {:?}", page_table.translate(4096 * 512 * 512 - 1));

    // Test map_to
    let addr = 4096 * 512 * 512 * 42; // 42th p3 entry
    let page = Page::containing_address(addr);
    let frame = allocator.allocate_frame()
        .expect("No more frames :(");
    println!("None = {:?}, map to {:?}",
             page_table.translate(addr),
             frame);
    page_table.map_to(page, frame, EntryFlags::empty(), allocator);
    println!("Some = {:?}", page_table.translate(addr));
    println!("Next free frame: {:?}", allocator.allocate_frame());

    // Test unmap
    page_table.unmap(Page::containing_address(addr), allocator);
    println!("None = {:?}", page_table.translate(addr));
}
