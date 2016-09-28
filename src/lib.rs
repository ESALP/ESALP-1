// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(lang_items)]
#![feature(alloc, collections)]
#![feature(const_fn, unique)]
#![feature(core_intrinsics)]
#![feature(associated_type_defaults)]
#![feature(naked_functions, asm)]
#![no_std]

// crates.io crates
extern crate rlibc;
extern crate spin;
extern crate multiboot2;
#[macro_use]
extern crate x86;
extern crate bit_field;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate once;

// Features involving allocation
extern crate hole_list_allocator;
extern crate alloc;
#[macro_use]
extern crate collections;

#[macro_use]
mod vga_buffer;
mod memory;
// This must be pub to expose functions to the linker
pub mod interrupts;

extern "C" {
    fn KEXIT() -> !;
}

#[no_mangle]
pub extern "C" fn rust_main(multiboot_info_address: usize) {
    vga_buffer::clear_screen();
    println!("Hello Rust log \x01");

    enable_nxe_bit();
    enable_write_protected_bit();

    let boot_info = unsafe {
        multiboot2::load(multiboot_info_address)
    };

    for module in boot_info.module_tags() {
        if module.name() == "keyboard" {
            unsafe {
                interrupts::KEYBOARD.lock()
                    .change_kbmap(&*(module.start_address() as u64 as *const [u8; 128]));
            }
        }
    }

    // Initialize the IDT
    interrupts::init();

    // Initialize memory
    memory::init(&boot_info);

    // Test allocation
    use alloc::boxed::Box;
    let heap_test = Box::new(42);

    println!("Try to write some things!");
    vga_buffer::WRITER.lock()
        .color(vga_buffer::Color::White, vga_buffer::Color::Black);

    loop {}
}

/// Enable the NXE bit in the CPU Extended Feature Register
fn enable_nxe_bit() {
    use x86::msr::{IA32_EFER,rdmsr, wrmsr};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

/// Enable the `WRITABLE` bit, it is ignored by default
fn enable_write_protected_bit() {
    use x86::controlregs::{cr0, cr0_write};

    let wp_bit = 1 << 16;

    unsafe { cr0_write(cr0() | wp_bit) }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    unsafe { KEXIT() }
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
#[lang = "panic_fmt"]
extern "C" fn panic_fmt(args: ::core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    vga_buffer::WRITER.lock().color(vga_buffer::Color::Red, vga_buffer::Color::Black);
    println!("\n\nPANIC at {}:{}", file, line);
    println!("\t{}", args);
    unsafe { KEXIT() }
}
