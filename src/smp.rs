// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use core::ptr::NonNull;

use x86_64::instructions::wrmsr;
use x86_64::registers::msr;

use sync::IrqLock;
use scheduler::Scheduler;

macro_rules! offset_of {
    ($ty:ty , $field:ident) => {
        &(*(0 as *const $ty)).$field as *const _ as usize
    }
}
macro_rules! read_gs_offset {
    ($offset:expr) => {{
        let ret: u64;
        asm!("mov $0, gs:$1" : "=r"(ret) : "i"($offset) : "memory" : "intel", "volatile");
        ret
    }}
}

/// ID of the next CPU to be initialized
static ID: AtomicUsize = ATOMIC_USIZE_INIT;

/// A structure that is unique to each CPU
// Some fields are only read through gs, so allow dead fields
#[allow(dead_code)]
pub struct CpuLocal {
    direct: NonNull<CpuLocal>,
    pub id: usize,
    pub sched: IrqLock<Scheduler>,
}

impl CpuLocal {
    fn new() -> CpuLocal {
        CpuLocal {
            direct: NonNull::dangling(),
            id: ID.fetch_add(1, Ordering::Relaxed),
            sched: IrqLock::new(Scheduler::new()),
        }
    }

    /// Initializes a `CpuLocal` structure for the current CPU
    ///
    /// Changes `GS.Base`
    pub unsafe fn init() {
        let ptr = Box::into_raw(Box::new(Self::new()));

        (*ptr).direct = NonNull::new(ptr).unwrap();

        // TODO assert msr & GS.base exist using cpuid
        wrmsr(msr::IA32_GS_BASE, ptr as u64);
    }
}

pub fn current() -> &'static CpuLocal {
    unsafe {
        &*(read_gs_offset!(offset_of!(CpuLocal, direct)) as *const CpuLocal)
    }
}

pub fn cpu_id() -> u32 {
    unsafe {
        read_gs_offset!(offset_of!(CpuLocal, id)) as u32
    }
}
