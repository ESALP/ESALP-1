// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_snake_case)]

#![feature(lang_items)]
#![feature(alloc)]
#![feature(const_fn)]
#![feature(associated_type_defaults)]
#![feature(asm, naked_functions, core_intrinsics)]
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
mod interrupts;
/// IO abstractions in Rust
#[macro_use]
mod cpuio;
mod sync;
mod scheduler;
/// Utilities for multi-CPU processing
mod smp;
/// Testing
#[cfg(feature = "test")]
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

    // Initialize CPU local variables and the scheduler
    unsafe {
        smp::CpuLocal::init()
    };

    // Initialize the IDT
    interrupts::init();

    // Initialize the serial port
    cpuio::init();

    println!("Try to write some things!");
    vga_buffer::change_color(vga_buffer::Color::White, vga_buffer::Color::Black);

    #[cfg(feature = "test")] {
        run_tests();
        shutdown();
    }

    loop {
        // We are waiting for interrupts here, so don't bother doing anything
        unsafe { asm!("hlt" :::: "volatile") }
    }
}

#[cfg(feature = "test")]
fn shutdown() -> ! {
    use cpuio::port::Port;
    let mut p: Port<u8> = unsafe { Port::new(0xf4) };
    p.write(0x00);
    unreachable!();
}


#[cfg(feature = "test")]
fn run_tests() {
    memory::tests::run();
    scheduler::tests::run();
    smp::tests::run();
    interrupts::tests::run();
    cpuio::tests::run();
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    unsafe { KEXIT() }
}

/// Used for unwinding, unsupported
#[lang = "eh_personality"]
fn eh_personality() {}


use core::alloc::Layout;
/// Runs when the allocator is out of memory
#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("Error, out of memory");
}

/// Runs during a `panic!()`
#[panic_handler]
extern "C" fn panic_fmt(pi: &core::panic::PanicInfo) -> ! {
    vga_buffer::change_color(vga_buffer::Color::Red, vga_buffer::Color::Black);
    println!("\n\nESALP {}", pi);

    #[cfg(feature = "test")] {
        serial_println!("Bail out! - {}", pi);
        shutdown();
    }

    unsafe { KEXIT() }
}
