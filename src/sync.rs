// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Synchronization primitives for ESALP

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut, Drop};

use interrupts;

/// While a lock for this struct is taken, interrutps are disabled
pub struct IrqLock<T: ?Sized> {
    inner: UnsafeCell<T>,
}

pub struct IrqGuard<'a, T: ?Sized + 'a> {
    data: &'a mut T,
    was_enabled: bool,
}

unsafe impl<T: ?Sized + Send> Sync for IrqLock<T> {}
unsafe impl<T: ?Sized + Send> Send for IrqLock<T> {}

impl<T> IrqLock<T> {
    pub const fn new(data: T) -> IrqLock<T> {
        IrqLock {
            inner: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> IrqGuard<T> {
        let enabled = interrupts::enabled();
        if enabled {
            unsafe { interrupts::disable() }
        }
        IrqGuard {
            data: unsafe { &mut *self.inner.get() },
            was_enabled: enabled,
        }
    }
}

impl<'a, T: ?Sized> Deref for IrqGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}
impl<'a, T: ?Sized> DerefMut for IrqGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for IrqGuard<'a, T> {
    fn drop(&mut self) {
        if self.was_enabled {
            unsafe { interrupts::enable() }
        }
    }
}
