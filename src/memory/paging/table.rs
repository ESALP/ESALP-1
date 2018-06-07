// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

// WARNING: IN ORDER FOR THIS FILE TO HAVE SAFETY
//          THE PAGE TABLE MUST BE MAPPED RECURSIVELY
//
// If the 510th entry of the p4 table is not mapped to
// the p4 table itself, we cannot gurentee that any of
// these addresses are valid.

use memory::paging::entry::*;
use memory::paging::ENTRY_COUNT;
use memory::paging::VirtualAddress;
use memory::FrameAllocate;

use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

/// A pointer to the level 4 page table
///
/// # Safety
/// This pointer is valid if and only if recursive mapping is valid.
pub const P4: *mut Table<Level4> = 0o177777_776_776_776_776_0000 as *mut _;

/// A page table
pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRY_COUNT],
    level: PhantomData<L>,
}

impl<L> Table<L>
    where L: TableLevel
{
    /// Sets each page table entry to unused
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
    
    pub fn clone_higher_half<A>(&self, allocator: &mut A) -> &mut Table<L>
        where A: FrameAllocate 
    {
        let frame = allocator.allocate_frame().expect("Unable to allocate frame.");
        
        let table: &mut Table<L> = unsafe { &mut *(frame.start_address() as *mut _) };
        // TODO: is this necessary?
        table.zero();

        let mut i = 0b1_0000_0000; // == 0x100 == 256
        while i < self.entries.len() {
            table[i] = self.entries[i].clone();
            i += 1;
        }
        table
    }
}

/// These methods can only be used if the given table is a parent to other tables.
impl<L> Table<L>
    where L: HierarchicalLevel
{
    /// Returns a reference to the table at index `index`.
    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|address| unsafe { &*(address as *const _) })
    }

    /// Returns a mutable reference to the table at index `index`
    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index)
            .map(|address| unsafe { &mut *(address as *mut _) })
    }

    /// Returns the address of the table at index `index`
    fn next_table_address(&self, index: usize) -> Option<VirtualAddress> {
        let entry_flags = self[index].flags();
        if entry_flags.contains(EntryFlags::PRESENT) && !entry_flags.contains(EntryFlags::HUGE_PAGE) {
            let table_address = self as *const _ as usize;
            // Or 0xffff << 48 to ensure a canonical address
            Some((0xffff << 48) | (table_address << 9) | (index << 12))
        } else {
            None
        }
    }

    /// If the next table does not exist, this function creates it with the physical
    /// frame allocator given and returns a mutable reference.
    pub fn next_table_create<A>(&mut self,
                                index: usize,
                                allocator: &mut A)
                                -> &mut Table<L::NextLevel>
        where A: FrameAllocate
    {
        if self.next_table(index).is_none() {
            assert!(!self.entries[index].flags().contains(EntryFlags::HUGE_PAGE),
                    "Mapping code does not support huge pages");
            let frame = allocator.allocate_frame()
                .expect("No frames availible :(");
            self.entries[index].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            self.next_table_mut(index).unwrap().zero();
            assert!(self.next_table_mut(index).is_some());
        }
        self.next_table_mut(index).unwrap()
    }
}

/// Allows indexing to be used on the `Table` type.
impl<L> Index<usize> for Table<L>
    where L: TableLevel
{
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

/// Allows mutable indexing to be used on the `Table` type.
impl<L> IndexMut<usize> for Table<L>
    where L: TableLevel
{
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

/// A trait that describes a table's level.
pub trait TableLevel {}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

/// Each table that is not `Level1` produces another table with lower level.
pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

impl HierarchicalLevel for Level4 {
    type NextLevel = Level3;
}
impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}
impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}
