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
use linked_list_allocator::Heap;

#[macro_use]
extern crate lazy_static;
extern crate linked_list_allocator;
extern crate spin;

pub const HEAP_START: usize = 0o_000_001_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

lazy_static! {
    static ref HEAP: Mutex<Heap> = Mutex::new(
        unsafe {
            Heap::new(HEAP_START, HEAP_SIZE)
        }
    );
}

#[no_mangle]
pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    HEAP.lock().allocate_first_fit(size, align)
        .expect("Out of memory")
}

#[no_mangle]
pub extern fn __rust_usable_size(size: usize, align: usize) -> usize {
    size
}

#[no_mangle]
pub extern fn __rust_deallocate(ptr: *mut u8, size: usize, align: usize) {
    unsafe { HEAP.lock().deallocate(ptr, size, align) }
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

