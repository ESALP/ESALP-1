// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use core::ops::{Deref, DerefMut};

use multiboot2::BootInformation;

pub use self::entry::*;
pub use self::mapper::Mapper;
use self::temporary_page::TemporaryPage;
use memory::{PAGE_SIZE, Frame, FrameAllocator};

mod entry;
mod table;
mod temporary_page;
mod mapper;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, Copy, Clone)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(usize);

impl Page {
    pub fn start_address(&self) -> usize {
        self.0 * PAGE_SIZE
    }

    pub fn containing_address(address: VirtualAddress) -> Page {
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

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start: start,
            end: end,
        }
    }
}

pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.0 += 1;
            Some(page)
        } else {
            None
        }
    }
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    /// Temporarly change the recursive mapping to the given table
    /// and executes the given closure in the new context.
    /// By return the table's state is restored.
    pub fn with<F>(&mut self,
                   table: &mut InactivePageTable,
                   temporary_page: &mut TemporaryPage,
                   f: F)
        where F: FnOnce(&mut Mapper)
    {
        use x86::{tlb, controlregs};
        let flush_tlb = || unsafe { tlb::flush_all() };

        {
            // Save table
            let backup = Frame::containing_address(
                // Safe iff the processor is in ring 0
                // during execution
                unsafe { controlregs::cr3() } as usize
            );

            // Map temporary_page to the current table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // Overwrite recursive mapping
            self.p4_mut()[511].set(table.p4_frame.clone(), PRESENT | WRITABLE);
            flush_tlb();

            // Execute the closure in the new context
            f(self);

            // Restore recursive mapping
            p4_table[511].set(backup, PRESENT | WRITABLE);
            flush_tlb();
        }

        temporary_page.unmap(self);
    }

    /// Activates the `InactivePageTable` given.
    /// Returns the previous ActivePageTable
    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86::controlregs;

        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(
                unsafe { controlregs::cr3() } as usize),
        };

        unsafe {
            controlregs::cr3_write(new_table.p4_frame.start_address() as u64);
        }
        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable,
               temporary_page: &mut TemporaryPage)
            -> InactivePageTable
    {
        {
            let table = temporary_page.map_table_frame(frame.clone(),
                                                       active_table);
            table.zero();
            // Now set up recursive mapping for the table
            table[511].set(frame.clone(), PRESENT | WRITABLE);
        }

        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}

pub fn remap_the_kernel<A>(allocator: &mut A, boot_info: &BootInformation)
    -> ActivePageTable
    where A: FrameAllocator
{
    // Create temporary page at arbritrary unused page address
    let mut temporary_page = TemporaryPage::new(Page(0xdeadbeef), allocator);

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame()
            .expect("No more frames");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info.elf_sections_tag()
            .expect("Memory map tag required");

        // Identity map the allocated kernel sections
        for (i, section) in elf_sections_tag.sections().enumerate() {
            if !section.is_allocated() {
                // Section is not loaded to memory
                continue;
            }
            assert!(section.addr as usize % PAGE_SIZE == 0,
                    "Section needs to be page aligned");

            let flags = EntryFlags::from_elf_section_flags(section);

            let start_frame = Frame::containing_address(section.start_address());
            let end_frame = Frame::containing_address(section.end_address() - 1);

            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.identity_map(frame, flags, allocator);
            }
        }

        // Identity map the VGA buffer
        let vga_buffer = Frame::containing_address(0xb8000);
        mapper.identity_map(vga_buffer, WRITABLE, allocator);

        // Identity map the multiboot info structure
        let multiboot_start = Frame::containing_address(boot_info.start_address());
        let multiboot_end = Frame::containing_address(boot_info.end_address() - 1);
        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            mapper.identity_map(frame, PRESENT, allocator);
        }
    });

    let old_table = active_table.switch(new_table);
    println!("New page table loaded");

    // Use the previous table as a guard page for the kernel stack
    let old_p4_page = Page::containing_address(
        old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator);

    println!("New guard page at {:#x}",old_p4_page.start_address());

    active_table
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
