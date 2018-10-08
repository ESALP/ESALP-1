// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![no_std]

extern crate linked_list_allocator;

use linked_list_allocator::LockedHeap;

// TODO use own mutex instead of spinlock in `LockedHeap`
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init(start: usize, size: usize) {
    ALLOCATOR.lock().init(start, size);
}
