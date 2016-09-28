// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(const_fn, allocator)]

#![allow(unused_variables)]

#![allocator]
#![no_std]

use spin::Mutex;

extern crate spin;

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 Kb

static BUMP_ALLOCATOR: Mutex<Allocator> = Mutex::new(
    Allocator::new(HEAP_START, HEAP_SIZE));


#[derive(Debug)]
struct Allocator{
    heap_start: usize,
    heap_size: usize,
    next: usize,
}

impl Allocator {
    /// Creates a new allocator which uses the memory in
    /// the range heap_start..heap_start + heap_size
    const fn new(heap_start: usize, heap_size: usize) -> Allocator {
        Allocator {
            heap_start: heap_start,
            heap_size: heap_size,
            next: heap_start,
        }
    }

    /// Allocates a block of memory with the given size and alignment
    fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let alloc_start = align_up(self.next, align);
        let alloc_end = alloc_start + size;

        if alloc_end <= self.heap_start + self.heap_size {
            self.next = alloc_end;
            Some(alloc_start as *mut u8)
        } else {
            None
        }
    }
}

/// Align downwards. Returns the greatest X with alignment `align`
/// such that x <= addr. The alignment must be a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        // bit fiddling is great :)
        addr & !(align - 1)
    } else if align == 0 {
        addr
    } else {
        panic!("Alignment must be power of two!");
    }
}

/// Align upwards. Returns the smalest x with alignment `align`
/// such that x >= addr. The alignment must be a power of 2.
pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}

// Here we implement the rust alloc functions

#[no_mangle]
pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    BUMP_ALLOCATOR.lock().allocate(size,align)
        .expect("Out of memory :(")
}

#[no_mangle]
pub extern fn __rust_usable_size(size: usize, align: usize) -> usize {
    size
}

#[no_mangle]
pub extern fn __rust_deallocate(ptr: *mut u8, size: usize, align: usize) {
    // Just leak it
}

#[no_mangle]
pub extern fn __rust_reallocate(ptr: *mut u8, size: usize,
                                new_size: usize, align: usize) -> *mut u8 
{
    use core::{ptr, cmp};

    let new_ptr = __rust_allocate(new_size, align);
    unsafe { ptr::copy(ptr, new_ptr, cmp::min(size, new_size)) };
    __rust_deallocate(ptr, size, align);
    new_ptr
}

#[no_mangle]
pub extern fn __rust_allocate_inplace(ptr: *mut u8, size: usize,
                                      new_size: usize, align: usize) -> usize 
{
    size
}

