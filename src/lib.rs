// Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
// file at the top-level directory of this distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(lang_items)]
#![feature(const_fn, unique)]
#![feature(associated_type_defaults)]
#![feature(asm)]
#![no_std]

extern crate rlibc;
extern crate spin;
extern crate multiboot2;
extern crate x86;
extern crate bit_field;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod vga_buffer;
mod memory;
pub mod interrupts;

extern "C" {
    fn KEXIT() -> !;
}

#[no_mangle]
pub extern "C" fn rust_main(multiboot_info_address: usize) {
    vga_buffer::clear_screen();
    println!("Hello Rust log \x01");

    let boot_info = unsafe {
        multiboot2::load(multiboot_info_address)
    };
    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required");
    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("Elf-sections tag required");

    let kernel_start = elf_sections_tag.sections()
        .map(|s| s.addr)
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag.sections()
        .map(|s| s.addr + s.size)
        .max()
        .unwrap();
    let multiboot_start = multiboot_info_address;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    for module in boot_info.module_tags() {
        if module.name() == "keyboard" {
            unsafe {
                interrupts::KEYBOARD.lock()
                    .change_kbmap(&*(module.start_address() as u64 as *const [u8; 128]));
            }
        }
    }

    enable_nxe_bit();

    // now create an allocator for memory
    let mut frame_allocator =
        memory::AreaFrameAllocator::new(kernel_start as usize,
                                        kernel_end as usize,
                                        multiboot_start as usize,
                                        multiboot_end as usize,
                                        memory_map_tag.memory_areas());

    memory::remap_the_kernel(&mut frame_allocator, boot_info);

    // Initialize the IDT
    interrupts::init();

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
