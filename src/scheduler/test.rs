// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

pub fn run() {
    serial_println!("Creating and yielding to new thread");
    super::add(new);
    super::thread_yield();
    serial_println!("Thread returned");
}

extern "C" fn new() {
    serial_println!("\tHello from new thread!");
    super::thread_yield();
}
