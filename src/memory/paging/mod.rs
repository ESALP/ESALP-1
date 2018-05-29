// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use core::ops::{Deref, DerefMut};
use core::ops::Add;

use multiboot2::{BootInformation, StringTable};
use spin::Mutex;

pub use self::entry::*;
pub use self::mapper::Mapper;
pub use self::temporary_page::{TemporaryPage, TinyAllocator};
use memory::{PAGE_SIZE, Frame, FrameAllocate};
use memory::{MemoryController, MEMORY_CONTROLLER};

use memory::frame_bitmap::FrameBitmap;

/// An entry in the page table.
mod entry;
/// Abstraction of the page table.
mod table;
/// A page to temporarily map a frame.
pub mod temporary_page;
/// An interface to the active page table.
mod mapper;

/// How many entries are in each table.
const ENTRY_COUNT: usize = 512;

/// This is the _only_ ActivePageTable that should be used in the system. Any others
/// would violate the assumptions of `Unique`.
pub static ACTIVE_TABLE: Mutex<ActivePageTable> = Mutex::new(unsafe {
    ActivePageTable::new()
});

// TODO FIXME Make conversions between the PhysicalAddress and VirtualAddress types
// unsafe. All addresses in this module have to be explicitly physical or virtual.
//
// Possibly make physical and virtual traits and apply them to numbers, pointers,
// etc.?
/// A _physical_ address on the machine. *_These should only be known to the kernel_*.
/// The userspace should never recieve a physical addresss.
pub type PhysicalAddress = usize;
/// A _virtual_ address on the machine. _All_ pointers are of this type. It is
/// undefined behaviour to convert a virtual to a physical address except through
/// the specific page table methods.
pub type VirtualAddress = usize;

/// A representation of a virtual page.
#[derive(Debug, Copy, Clone)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(usize);

impl Page {
    /// Returns the first address in the `Page`
    pub fn start_address(&self) -> VirtualAddress {
        self.0 * PAGE_SIZE
    }

    /// Returns a `Page` containing the VirtualAddress given
    ///
    /// # Panics
    /// A panic will occur if the address is not canonical. For the address
    /// to be canonical the first 48 bits must be sign extended.
    pub fn containing_address(address: VirtualAddress) -> Page {
        // Address must be canonical
        assert!(address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
                "invalid address: 0x{:x}",
                address);
        Page(address / PAGE_SIZE)
    }

    /// Returns `Page`'s index into the p4 table.
    ///
    /// The value must be from 0 to `ENTRY_COUNT`
    fn p4_index(&self) -> usize {
        (self.0 >> 27) & 0o777
    }

    /// Returns `Page`'s index into the p3 table.
    ///
    /// The value must be from 0 to `ENTRY_COUNT`
    fn p3_index(&self) -> usize {
        (self.0 >> 18) & 0o777
    }

    /// Returns `Page`'s index into the p2 table.
    ///
    /// The value must be from 0 to `ENTRY_COUNT`
    fn p2_index(&self) -> usize {
        (self.0 >> 9) & 0o777
    }

    /// Returns `Page`'s index into the p1 table.
    ///
    /// The value must be from 0 to `ENTRY_COUNT`
    fn p1_index(&self) -> usize {
        (self.0 >> 0) & 0o777
    }

    /// Returns a `Page` iterator from `start` to `end`.
    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start: start,
            end: end,
        }
    }

    /// Map this `Page` to the given `Frame`
    pub fn map_to(self, frame: Frame, flags: EntryFlags) {
        let mut lock = MEMORY_CONTROLLER.lock();
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            stack_allocator: _,
        } = lock.as_mut().unwrap();
        active_table.map_to(self, frame, flags, frame_allocator)
            .expect("Unable to map frame because page is already taken");
    }

    /// Map this `Page` to any availible `Frame`.
    ///
    /// # Panics
    /// Panics if OOM
    pub fn map(self, flags: EntryFlags) {
        let mut lock = MEMORY_CONTROLLER.lock();
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            stack_allocator: _,
        } = lock.as_mut().unwrap();
        active_table.map(self, flags, frame_allocator);

    }

    /// Unmap this `Page`
    pub fn unmap(self) {
        let mut lock = MEMORY_CONTROLLER.lock();
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            stack_allocator: _,
        } = lock.as_mut().unwrap();

        active_table.unmap(self, frame_allocator);
    }
}

impl Add<usize> for Page {
    type Output = Page;

    fn add(self, rhs: usize) -> Page {
        Page(self.0 + rhs)
    }
}

/// Identity map the given `Frame`
fn identity_map(frame: Frame, flags: EntryFlags) {
        let page = Page::containing_address(frame.start_address());
        page.map_to(frame, flags);
}

/// An iterator across `Page`s
#[derive(Clone)]
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

/// An abstraction to the Active table, dereferences to the `Mapper` type.
pub struct ActivePageTable {
    pub mapper: Mapper,
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
    /// Creates a new `ActivePageTable`.
    ///
    /// # Safety
    /// The page table must be recursively mapped, if it is not any methods using
    /// the active page table will most likely produce undefined behaviour.
    pub const unsafe fn new() -> ActivePageTable {
        ActivePageTable { mapper: Mapper::new() }
    }

    /// Temporarily change the recursive mapping to the given table
    /// and execute the given closure in the new context.
    /// By return the table's state is restored.
    pub fn with<F>(&mut self,
                   table: &mut InactivePageTable,
                   temporary_page: &mut TemporaryPage,
                   f: F)
        where F: FnOnce(&mut Mapper)
    {
        use x86_64::registers::control_regs;
        use x86_64::instructions::tlb;

        {
            // Save table
            let backup = Frame::containing_address(control_regs::cr3().0 as usize);

            // Map temporary_page to the current table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // Overwrite recursive mapping
            self.p4_mut()[510].set(table.p4_frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();

            // Execute the closure in the new context
            f(self);

            // Restore recursive mapping
            p4_table[510].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }

        temporary_page.unmap(self);
        temporary_page.allocator.flush(|_| {});
    }

    /// Activates the `InactivePageTable` given.
    /// Returns the previous ActivePageTable
    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86_64::PhysicalAddress;
        use x86_64::registers::control_regs;

        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(control_regs::cr3().0 as usize),
        };

        unsafe {
            control_regs::cr3_write(PhysicalAddress(new_table.p4_frame.start_address() as u64));
        }
        old_table
    }
}

/// A level 4 table that is not yet used
pub struct InactivePageTable {
    pub p4_frame: Frame,
}

impl InactivePageTable {
    /// Creates a new `InactivePageTable`
    ///
    /// The `frame` is consumed and used to hold the inactive level 4 table. The table
    /// that is returned has recursive mapping, so activating it is safe.
    pub fn new(frame: Frame,
               active_table: &mut ActivePageTable,
               temporary_page: &mut TemporaryPage)
               -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_table);
            // Now that it's mapped we can zero it
            table.zero();
            // Now set up recursive mapping for the table
            table[510].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }

        temporary_page.unmap(active_table);
        // At this point, the p4 table still remains in the allocator, flush it
        // to make sure that the `InactivePageTable` is the only owner
        temporary_page.allocator.flush(|_| {});

        InactivePageTable { p4_frame: frame }
    }
}

/// Remaps the kernel using the given `ActivePageTable`
///
/// Each kernel section is mapped to the higher half with the correct permissions.
/// This function also identity maps the VGA text buffer and maps the multiboot2
/// information structure to the higher half.
pub fn remap_the_kernel<FA>(active_table: &mut ActivePageTable,
                            mut allocator: FA,
                            boot_info: &BootInformation) -> FrameBitmap
    where FA: FrameAllocate
{
    use memory::KERNEL_BASE;

    // Create new inactive table using a temporary page
    let mut temporary_page = 
        TemporaryPage::new(Page(0xdeadbeef), &mut allocator);
    let mut new_table = {
        let frame = allocator.allocate_frame()
            .expect("No more frames");
        InactivePageTable::new(frame, &mut ACTIVE_TABLE.lock(), &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {

        let elf_sections_tag = boot_info.elf_sections_tag()
            .expect("Memory map tag required");

        let string_table = unsafe {
            &*((elf_sections_tag.string_table() as *const StringTable).offset(KERNEL_BASE as isize))
        };

        // Map the allocated kernel sections to the higher half
        for section in elf_sections_tag.sections() {
            if !section.is_allocated() {
                // Section is not loaded to memory
                continue;
            }
            if string_table.section_name(&section) == ".init" {
                // We do not map the init section because it is not
                // used after boot
                // FIXME do not leak these frames
                continue;
            }
            assert!(section.addr as usize % PAGE_SIZE == 0,
                    "Section needs to be page aligned");

            let flags = EntryFlags::from_elf_section_flags(section);

            let start_frame = Frame::containing_address(section.start_address() - KERNEL_BASE);
            let end_frame = Frame::containing_address((section.end_address() - KERNEL_BASE) - 2);

            for frame in Frame::range_inclusive(start_frame, end_frame) {
                let new_page = Page::containing_address(frame.start_address() + KERNEL_BASE);
                mapper.map_to(new_page, frame, flags, &mut allocator).expect("Unable to map elf section frame");
            }
        }

        // Identity map the VGA buffer
        let vga_buffer = Frame::containing_address(0xb8000);
        mapper.identity_map(vga_buffer, EntryFlags::WRITABLE, &mut allocator);

        // Map the multiboot info structure to the higher half
        let multiboot_start = Frame::containing_address(boot_info.start_address() - KERNEL_BASE);
        let multiboot_end = Frame::containing_address((boot_info.end_address() - KERNEL_BASE) - 1);

        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            let new_page = Page::containing_address(frame.start_address() + KERNEL_BASE);
            // if we have already mapped this page, it must have been
            // already mapped when we mapped the elf sections.
            let _ = mapper.map_to(new_page, frame, EntryFlags::PRESENT, &mut allocator);
        }
    });
    let old_table = active_table.switch(new_table);
    println!("New page table loaded");

    // Now, we're done allocating and need a struct with FrameDeallocate. Init
    // the FrameBitmap
    let mut frame_bitmap = FrameBitmap::new(allocator, active_table);

    temporary_page.consume(&mut frame_bitmap);

    // Use the previous table as a guard page for the kernel stack
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address() + KERNEL_BASE);
    active_table.unmap(old_p4_page, &mut frame_bitmap);

    println!("New guard page at {:#x}", old_p4_page.start_address());

    frame_bitmap
}

#[cfg(feature = "test")]
pub mod tests {

    use memory::{MemoryController,MEMORY_CONTROLLER,FrameAllocate};
    use super::Page;
    use super::entry::EntryFlags;
    use tap::TestGroup;

    pub fn run() {
        let mut lock = MEMORY_CONTROLLER.lock();
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            stack_allocator: _,
        } = lock.as_mut().unwrap();

        let mut tap = TestGroup::new(7);
        tap.diagnostic("Testing page table mappings");
        // Address 0 should not be mappd
        tap.assert_tap(active_table.mapper.translate(0).is_none(),
                       "Address 0 mapped");

        // Page table should be mapped
        tap.assert_tap(
            active_table.mapper.translate(super::table::P4 as usize).is_some(),
            "Page table not recursively mapped!");

        // Heap should be mapped (check first page)
        tap.assert_tap(
            active_table.mapper.translate(::memory::HEAP_START,).is_some(),
            "Heap not mapped!");

        // frame bitmap should be mapped (check first page)
        tap.assert_tap(
            active_table.mapper.translate(::memory::frame_bitmap::BITMAP_BASE)
                .is_some(), "Frame bitmap not mapped!");

        tap.diagnostic("Testing `map_to`");
        // Test map_to
        let addr = 4096 * 512 * 512 * 12; // 12th p3 entry
        let page = Page::containing_address(addr);
        let frame = frame_allocator.allocate_frame()
            .expect("No more frames :(");
        tap.assert_tap(active_table.mapper.translate(addr).is_none(),
                 "Test page (12th P3), was unexpecteldly already mapped");
        let res = active_table.map_to(page, frame, EntryFlags::empty(), frame_allocator);
        tap.assert_tap(res.is_ok(),
                       "Unable to successfully use map_to() to map 12th P3 entry");

        tap.diagnostic("Testing `unmap`");
        // Test unmap
        active_table.unmap(Page::containing_address(addr), frame_allocator);
        tap.assert_tap(active_table.mapper.translate(addr).is_none(), 
                       "Did non successfully unmap test page (12th P3)");
    }

}
