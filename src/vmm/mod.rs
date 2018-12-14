// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

// TODO remove
#![allow(dead_code,unused_variables)]

use multiboot2::BootInformation;
use spin::Mutex;

use core::mem::MaybeUninit;
use alloc::collections::linked_list::LinkedList;

use arch::mem::ArchSpecificVMM;
pub use arch::mem::{KERNEL_SPACE_START, KERNEL_SPACE_END};
pub use arch::mem::PAGE_SIZE;
use arch::mem::{arch_vmm_init_preheap, arch_vmm_init};
use arch::mem::{arch_map_to, arch_map, arch_unmap};
use arch::mem::arch_alloc_stack;
pub use arch::mem::Stack;

// TODO export from arch
pub type Vaddr = usize;
pub type Paddr = usize;

/// The only current VMM
static KERNEL_VMM: Mutex<MaybeUninit<VMM>> = Mutex::new(MaybeUninit::uninitialized());

/// Initialize virtual memory
pub fn vm_init(boot_info: &BootInformation) {
    assert_has_not_been_called!("vmm::vm_init must only be called once!");

    let arch_specific = arch_vmm_init_preheap(boot_info);
    // heap works at this point
    let mut vmm = VMM {
        start: KERNEL_SPACE_START,
        regions: LinkedList::new(),
        arch_specific: arch_specific,
        end: KERNEL_SPACE_END,
    };
    //add arch specific regions
    arch_vmm_init(&mut vmm);

    KERNEL_VMM.lock().set(vmm);
}

/// Errors which can occur when mapping or unmapping memory
#[derive(Debug)]
pub enum VmmError {
    MemUsed,
    PhysMemUsed,
    OOM
}

/// Map `region` to the paddr `start_address` or return an error
pub fn map_to(region: Region, start_address: Paddr) -> Result<(),VmmError> {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };
    if !vmm.insert(region) {
        return Err(VmmError::MemUsed);
    }
    if let Err(E) = arch_map_to(&mut vmm.arch_specific, region, start_address) {
        vmm.remove_region(region.start);
        return Err(E)
    }
    Ok(())
}

/// Map `region` or return an error
pub fn map(region: Region) -> Result<(),VmmError> {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };

    if !vmm.insert(region) {
        return Err(VmmError::MemUsed);
    }
    if let Err(E) = arch_map(&mut vmm.arch_specific, region) {
        vmm.remove_region(region.start);
        return Err(E)
    }
    Ok(())
}

/// Unmap the region associated with `addr`
/// Returns `true` iff a region was unmapped
// TODO make it posssible to unmap a region
pub fn unmap(addr: Vaddr) -> bool {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };

    if let Some(region) = vmm.remove_region(addr) {
        arch_unmap(&mut vmm.arch_specific, region);
        true
    } else {
        false
    }
}

/// Allocates a stack of `size` pages
// TODO fix stacks
pub fn alloc_stack(size: usize) -> Result<Stack, &'static str> {
    let mut vmm_lock = KERNEL_VMM.lock();
    let vmm = unsafe { vmm_lock.get_mut() };

    // TODO rewrite and remove arch specific
    arch_alloc_stack(&mut vmm.arch_specific, size)
}

pub struct VMM {
    start: Vaddr,
    regions: LinkedList<Region>,
    // TODO fix visability annotations
    pub arch_specific: ArchSpecificVMM,
    end: Vaddr,
}

impl VMM {
    /// Insert `region` into the VMM. Returns `false` if it intersects with an
    /// existing region.
    ///
    /// # Safety
    /// The inserted region is not actually mapped into memory.
    pub fn insert(&mut self, region: Region) -> bool {
        let mut iter = self.regions.iter_mut();
        loop {
            if let Some(next_region) = iter.peek_next() {
                match region.relation(next_region) {
                    RegionOrder::Less => (),
                    RegionOrder::Greater => break,
                    RegionOrder::Intersects => return false,
                }
            } else {
                break;
            }
            iter.next();
        }
        iter.insert_next(region);
        return true;
    }

   /// Returns the region that contains `address`, if it exits
   pub fn containing_region(&self, address: Vaddr) -> Option<Region> {
       self.regions.iter().filter(|region| region.contains(address))
           // should contain /at most/ one region
           .next().cloned()
   }

   /// Remove the region intersecting with `address`
   pub fn remove_region(&mut self, address: usize) -> Option<Region>
   {
       self.regions.drain_filter(|region| region.contains(address))
           // should contain /at most/ one region
           .next()
   }
}

#[derive(Clone,Copy)]
#[derive(Debug)]
pub struct Region {
    pub name: &'static str,
    pub start: Vaddr,
    pub end: Vaddr,
    pub protection: Protection,
}

bitflags! {
    /// Flags that are used in the entry.
    pub struct Protection: usize {
        const NONE            = 0;
        const WRITABLE        = 1 << 0;
        const USER_ACCESSIBLE = 1 << 1;
        const EXECUTABLE      = 1 << 2;
        // TODO COW
    }
}

#[derive(PartialEq)]
enum RegionOrder {
    Less,
    Greater,
    Intersects,
}

impl Region {
    pub fn new(name: &'static str, start: Vaddr, end: Vaddr, protection: Protection) -> Region {
        Region {
            name: name,
            start: start,
            end: end,
            protection: protection,
        }
    }

    fn contains(&self, addr: Vaddr) -> bool {
        addr >= self.start && addr <= self.end
    }

    /// Returns true iff the regions intersect
    fn intersects(&self, other: &Self) -> bool {
        self.relation(other) == RegionOrder::Intersects
    }

    fn relation(&self, other: &Self) -> RegionOrder {
        if self.end < other.start {
            RegionOrder::Less
        } else if self.start > other.end {
            RegionOrder::Greater
        } else {
            RegionOrder::Intersects
        }
    }

    // TODO unmap
    //fn difference(self, other: &Self) -> Option<(Region,Option<Region>)> {
    //}
}

#[cfg(feature = "test")]
pub mod tests {
    use tap::TestGroup;
    use super::{Region, map, unmap, Vaddr, Protection};

    pub fn run() {
        let mut tap = TestGroup::new(3);

        tap.diagnostic("Testing vmm");
        // Test map_to

        // random address (chosen by fair dice roll)
        let addr: Vaddr = 0x100000;
        let region = Region::new("Test region", addr, addr + (5 * super::PAGE_SIZE),
            Protection::WRITABLE);
        tap.assert_tap(map(region).is_ok(),
            "Could not map region");
        unsafe {
            let a = addr as *mut usize;
            *a = 0;
            tap.assert_tap(*a == 0, "Could not read from mapped region");
        }
        // Test unmap

        tap.assert_tap(unmap(addr), "Could not unmap new region");
    }
}
