// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_snake_case)]

#![feature(const_unsafe_cell_new)]
#![feature(const_atomic_usize_new)]
#![feature(const_unique_new)]

#![feature(lang_items)]
#![feature(alloc)]
#![feature(const_fn, unique)]
#![feature(associated_type_defaults)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![no_std]

// crates.io crates
extern crate rlibc;
/// Bare metal Mutex
extern crate spin;
/// Abstraction of the multiboot2 info structure
extern crate multiboot2;
extern crate x86_64;
extern crate bit_field;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
/// A macro for running a function only once
#[macro_use]
extern crate once;

// Features involving allocation
/// Heap allocator for rust code
extern crate hole_list_allocator;
/// Higher-level data structures that use the heap
extern crate alloc;

#[macro_use]
/// Abstraction of the VGA text buffer
mod vga_buffer;
/// Memory management
mod memory;
/// Interrupts code
// This must be pub to expose functions to the linker
pub mod interrupts;
/// IO abstractions in Rust
mod cpuio;
/// Testing
mod tap;

extern "C" {
    /// The kernel exit point. It disables interrupts, enters an infinite loop,
    /// and halts the processor
    fn KEXIT() -> !;
}

/// The Rust entry point
///
/// This clears the screen, initializes each module and enters an infinite
/// loop.
#[no_mangle]
pub extern "C" fn rust_main(multiboot_info_address: usize) -> ! {
    vga_buffer::clear_screen();
    println!("Hello Rust log \x01");

    let boot_info = unsafe { multiboot2::load(multiboot_info_address) };

    for module in boot_info.module_tags() {
        if module.name() == "keyboard" {
            let addr = module.start_address() as usize + memory::KERNEL_BASE;
            unsafe {
                interrupts::KEYBOARD.lock()
                    .change_kbmap(&*(addr as *const [u8; 128]));
            }
        }
    }

    // Initialize memory
    memory::init(&boot_info);

    // Initialize the IDT
    interrupts::init();

    // Initialize the serial port
    cpuio::init();

    println!("Try to write some things!");
    vga_buffer::change_color(vga_buffer::Color::White, vga_buffer::Color::Black);

    run_tests();

    loop {
        // We are waiting for interrupts here, so don't bother doing anything
        unsafe { asm!("hlt") }
    }
}

pub fn run_tests() {
    //vga_buffer::run_tests();
    memory::run_tests();
    //interrupts::run_tests();
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    unsafe { KEXIT() }
}

/// Used for unwinding, unsupported
#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {}


/// Runs when the allocator is out of memory
#[lang = "oom"]
fn oom() -> ! {
    panic!("Error, out of memory");
}


/// Runs during a `panic!()`
#[no_mangle]
#[lang = "panic_fmt"]
pub extern "C" fn panic_fmt(args: ::core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    vga_buffer::change_color(vga_buffer::Color::Red, vga_buffer::Color::Black);
    println!("\n\nPANIC at {}:{}", file, line);
    println!("\t{}", args);
    unsafe { KEXIT() }
}
