// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use bit_field::BitField;

/// The type of an interrupt service routine
pub type HandlerFunc = unsafe extern "C" fn();

/// The Interrupt Descriptor table
///
/// # Safety
/// The `IDT` must be CPU-local.
pub struct Idt([Entry; 40]);

impl Idt {
    /// Creates a new Idt, each entry is unused
    pub const fn new() -> Idt {
        Idt([Entry::missing(); 40])
    }

    /// Sets a certain entry as present and initializes it with a certain handler
    pub fn set_handler(&mut self, entry: u8, handler: HandlerFunc) -> &mut EntryOptions {
        use x86_64::instructions::segmentation;

        self.0[entry as usize] = Entry::new(segmentation::cs().0, handler);
        &mut self.0[entry as usize].options
    }

    /// Loads the given IDT into the CPU
    ///
    /// # Safety
    /// The IDT must be valid, if it is not undefined behaviour will most likely
    /// occur. `self` must live for the duration of the kernel, however, as
    /// long as it is only mutated while interrupts are disabled, access is safe.
    /// For this reason we do not currently use 'static, as it would disallow any
    /// mutable references.
    // XXX There are two ways to do this. Either make the mutable functions
    // take a &self and disable interrupts with them, or make them take a
    // &mut self, and remove static on this function. Both will work, but I
    // took the latter approach this time as this function is marked unsafe
    pub unsafe fn load(&self) {
        use x86_64::instructions::tables::{lidt, DescriptorTablePointer};
        use core::mem::size_of;

        let ptr = DescriptorTablePointer {
            base: self as *const _ as u64,
            limit: (size_of::<Self>() - 1) as u16,
        };
        lidt(&ptr)
    }

    pub fn get_handler(&self, entry: u8) -> Entry {
        self.0[entry as usize]
    }
}

/// An Interrupt Descriptor Table entry type
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Entry {
    /// The lowest 16 bits of the isr pointer
    pointer_low: u16,
    gdt_selector: u16,
    /// A bitfield of options for the entry
    options: EntryOptions,
    /// The middle 16 bits of the isr pointer
    pointer_middle: u16,
    /// The high 32 bits of the isr pointer
    pointer_high: u32,
    /// Must be zero
    reserved: u32,
}

impl Entry {
    /// Creates a new Entry given a `HandlerFunc`
    fn new(gdt_selector_index: u16, handler: HandlerFunc) -> Self {
        let pointer = handler as usize;
        Entry {
            gdt_selector: gdt_selector_index,
            pointer_low: pointer as u16,
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            options: EntryOptions::new(),
            reserved: 0,
        }
    }

    /// Returns an uninitialized Entry
    const fn missing() -> Self {
        Entry {
            gdt_selector: 0,
            pointer_low: 0,
            pointer_middle: 0,
            pointer_high: 0,
            options: EntryOptions::minimal(),
            reserved: 0,
        }
    }

    pub fn func(self) -> Option<HandlerFunc> {
        use core::mem;
        if self.options.is_present() {
            let pointer = ((self.pointer_high as usize) << 32)
                | ((self.pointer_middle as usize) << 16)
                | (self.pointer_low as usize);
            // XXX only `transmute` usage in the kernel so far, replace with
            // something better if possible
            Some(unsafe { mem::transmute::<usize, HandlerFunc>(pointer) })
        } else {
            None
        }
    }

    pub fn options(&self) -> &EntryOptions {
        &self.options
    }

}

/// A representation of the `Options` field of an `Entry`
#[derive(Debug, Clone, Copy)]
pub struct EntryOptions(u16);

impl EntryOptions {
    /// Returns options with 'must be one' bits set
    const fn minimal() -> Self {
        EntryOptions(0b111 << 9) // 'must be one' bits
    }

    /// Returns a option with `present` and `disable_interrupts` set
    fn new() -> Self {
        let mut options = Self::minimal();
        options.set_present(true);
        options.disable_interrupts(true);
        options
    }

    /// Sets or resets the `present` bit
    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }

    fn is_present(&self) -> bool {
        self.0.get_bit(15)
    }


    /// Let the CPU disable hardware interrupts when the handler is invoked.
    /// By default, interrupts are disabled.
    pub fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0.set_bit(8, !disable);
        self
    }

    /// Sets the required privilege level(DPL) for invoking the handler. If
    /// CPL > DPL, a #GP occurs.
    ///
    /// # Panics
    /// Panic if DPL > 3
    pub fn set_privilege_level(&mut self, dpl: u16) -> &mut Self {
        self.0.set_bits(13..15, dpl);
        self
    }

    /// Assigns a Interrupt Stack Table (IST) stack to this handler. The CPU
    /// will then always switch to the specified stack before the handler is
    /// invoked. This allows kernels to recover from corrupt stack pointers
    /// (e.g., on kernel stack overflow).
    ///
    /// An IST stack is specified by an IST index between 0 and 6 (inclusive).
    /// Using the same stack for multiple interrupts can be dangerous when
    /// nested interrupts are possible.
    ///
    /// This function panics if the index is not in the range 0..7.
    ///
    /// ## Safety
    /// This function is unsafe because the caller must ensure that the passed
    /// stack index is valid and not used by other interrupts. Otherwise, memory
    /// safety violations are possible.
    pub unsafe fn set_stack_index(&mut self, index: u16) -> &mut Self {
        // The hardware IST index starts at 1, but our software IST index
        // starts at 0. Therefore we need to add 1 here.
        self.0.set_bits(0..3, index + 1);
        self
    }

    /// Get the TSS stack index for the given options if it exists
    pub fn get_stack_index(&self) -> Option<u16> {
        self.0.get_bits(0..3).checked_sub(1)
    }
}
