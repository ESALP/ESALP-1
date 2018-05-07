// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Reading and writing to CPU ports in Rust

#![allow(dead_code)]
use core::marker::PhantomData;

extern "C" {
    /// Reads one byte from `port`
    fn inb(port: u16) -> u8;
    /// Outputs one byte to `port`
    fn outb(port: u16, value: u8);
    /// Reads one word from `port`
    fn inw(port: u16) -> u16;
    /// Outputs one word to `port`
    fn outw(port: u16, value: u16);
    /// Reads one long from `port`
    fn inl(port: u16) -> u32;
    /// Outputs one long to `port`
    fn outl(port: u16, value: u32);
}

/// A type implements `InOut` if it can be written to and read from a port
pub trait InOut {
    /// Read one `Self` from port
    unsafe fn port_in(port: u16) -> Self;
    /// Write one `Self` from port
    unsafe fn port_out(port: u16, value: Self);
}

impl InOut for u8 {
    unsafe fn port_in(port: u16) -> u8 {
        inb(port)
    }
    unsafe fn port_out(port: u16, value: u8) {
        outb(port, value);
    }
}

impl InOut for u16 {
    unsafe fn port_in(port: u16) -> u16 {
        inw(port)
    }
    unsafe fn port_out(port: u16, value: u16) {
        outw(port, value);
    }
}

impl InOut for u32 {
    unsafe fn port_in(port: u16) -> u32 {
        inl(port)
    }
    unsafe fn port_out(port: u16, value: u32) {
        outl(port, value);
    }
}

// TODO enable safety checking in new()
/// An abstraction of a port for T
pub struct Port<T> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    /// Creates a new port with the given port number
    ///
    /// # Safety
    /// Some ports are completely harmless to access, some could brick the
    /// machine. It is up to calling code to provide a safe abstraction of
    /// a hardware port
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port {
            port: port,
            phantom: PhantomData,
        }
    }

    /// Reads one `T` from the port
    pub fn read(&mut self) -> T {
        unsafe { T::port_in(self.port) }
    }

    /// Writes one `T` to the port
    pub fn write(&mut self, value: T) {
        unsafe { T::port_out(self.port, value) }
    }
}

/// Exactly the same as Port<T>, but with more explicit safety
pub struct UnsafePort<T> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> UnsafePort<T> {
    /// Creates a new port with the given port number
    ///
    /// # Safety
    /// Some ports are completely harmless to access, some could brick the
    /// machine. It is up to calling code to provide a safe abstraction of
    /// a hardware port
    pub const unsafe fn new(port: u16) -> UnsafePort<T> {
        UnsafePort {
            port: port,
            phantom: PhantomData,
        }
    }

    /// Reads one `T` from the port
    ///
    /// # Safety
    /// Depending on the port number this could produce undefined values.
    pub unsafe fn read(&mut self) -> T {
        T::port_in(self.port)
    }

    /// Writes one `T` to the port
    ///
    /// # Safety
    /// This operation could put the machine in an undefined state depending
    /// on the port number.
    pub unsafe fn write(&mut self, value: T) {
        T::port_out(self.port, value)
    }
}
