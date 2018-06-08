// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use multiboot2::ElfSection;

use memory::Frame;

/// A representation of a page table entry.
#[derive(Clone,PartialEq,Debug)]
pub struct Entry(u64);

impl Entry {
    /// Checks if an entry is unused.
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    /// Zeros an entry, setting it to unused.
    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    /// Gets the Entry flags.
    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    /// Returns the `Frame` that it is mapping.
    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            Some(Frame::containing_address(self.0 as usize & 0x000fffff_fffff000))
        } else {
            None
        }
    }

    /// Sets the entry to point to the given `frame` with given flags.
    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(frame.start_address() & !0x000fffff_fffff000 == 0);
        self.0 = (frame.start_address() as u64) | flags.bits();
    }
}

bitflags! {
    /// Flags that are used in the entry.
    pub struct EntryFlags: u64 {
        /// Page exists in the table
        const PRESENT =         1 << 0;
        /// If set, R/W, else read-only
        const WRITABLE =        1 << 1;
        /// If set, accessible by all rings
        const USER_ACCESSIBLE = 1 << 2;
        /// If set, writes will go straight to ram rather than cache
        /// (this is useful for memory-mapped hardware)
        const WRITE_THROUGH =   1 << 3;
        /// Altogether disable caching
        const CACHE_DISABLED =  1 << 4;
        /// Set if the page is read or written
        /// (not cleared by hardware)
        const ACCESSED =        1 << 5;
        /// Set if page is written to
        const DIRTY =           1 << 6;
        /// If set, pages are 4MiB large. Otherwise they are 4KiB large
        const HUGE_PAGE =       1 << 7;
        /// Prevent the CPU from updating this page on a CR3 change.
        /// (use for kernel pages)
        const GLOBAL =          1 << 8;
        /// If set, fault on execution of the page
        const NO_EXECUTE =      1 << 63;
    }
}

impl EntryFlags {
    /// Returns the `EntryFlags` that are required for the given `ElfSection`'s
    /// permissions
    pub fn from_elf_section_flags(section: &ElfSection) -> EntryFlags {
        use multiboot2::{ELF_SECTION_ALLOCATED, ELF_SECTION_WRITABLE, ELF_SECTION_EXECUTABLE};

        let mut flags = Self::empty();

        if section.flags().contains(ELF_SECTION_ALLOCATED) {
            flags |= Self::PRESENT;
        }
        if section.flags().contains(ELF_SECTION_WRITABLE) {
            flags |= Self::WRITABLE;
        }
        if !section.flags().contains(ELF_SECTION_EXECUTABLE) {
            flags |= Self::NO_EXECUTE;
        }

        flags
    }
}
