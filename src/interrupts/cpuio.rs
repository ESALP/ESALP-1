// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]
use core::marker::PhantomData;

extern "C" {
    fn inb(port: u16) -> u8;
    fn outb(port: u16, value: u8);
    fn inw(port: u16) -> u16;
    fn outw(port: u16, value: u16);
    fn inl(port: u16) -> u32;
    fn outl(port: u16, value: u32);
}

pub trait InOut {
    unsafe fn port_in(port: u16) -> Self;
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
pub struct Port<T> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port {
            port: port,
            phantom: PhantomData,
        }
    }
    pub fn read(&mut self) -> T {
        unsafe { T::port_in(self.port) }
    }
    pub fn write(&mut self, value: T) {
        unsafe { T::port_out(self.port, value) }
    }
}

pub struct UnsafePort<T> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> UnsafePort<T> {
    pub const unsafe fn new(port: u16) -> UnsafePort<T> {
        UnsafePort {
            port: port,
            phantom: PhantomData,
        }
    }
    pub unsafe fn read(&mut self) -> T {
        T::port_in(self.port)
    }
    pub unsafe fn write(&mut self, value: T) {
        T::port_out(self.port, value)
    }
}
