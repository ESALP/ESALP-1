// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use interrupts::{Context, EXIT_INT};
use memory::{alloc_stack, Stack};
use core::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use core::mem;

/// The `id` of the next thread to be created
static ID: AtomicUsize = ATOMIC_USIZE_INIT;
/// The basic number of "ticks" each program gets to run
pub const TICKS: u8 = 10;

extern "C" {
    static kstack_late_bottom: usize;
    static kstack_top: usize;
}

pub enum State {
    Running,
    Ready,
    Sleeping,
}

pub struct KThread {
    pub id: usize,
    stack: Stack,
    // Ready => Some(_), _ => None
    // XXX should this be a &'static _ or *const _ ? The former is wrong but
    // works and the latter is cumbersome but more explicit.
    // This is the top of the kernel stack when the thread is queued, and
    // `None`when it is running.
    context: Option<&'static Context>,
    pub quanta: u8,
    pub state: State,
}

impl KThread {
    /// Create a new thread with the given start point
    ///
    /// # Side effects
    /// Allocates a global stack for the given thread
    pub fn new(start: extern "C" fn()) -> Result<KThread, &'static str> {
        // for now create a 1-page stack
        let stack = alloc_stack(1)?;
        // now we must put the things we need on the stack.
        // In the meanwhile, grab an unbounded lifetime to our context
        let context = unsafe {
            // if the function ever returns, make it go to the thread exit point
            let mut stack_pointer = stack.top();
            stack_pointer -= mem::size_of::<extern "C" fn() -> !>();
            *(stack_pointer as *mut extern "C" fn() -> !) = exit;

            // Now we put on a fake interrupt context for returning to the thread
            let context_pointer = stack_pointer - mem::size_of::<Context>();
            {
                let context = (context_pointer as *mut Context).as_mut().unwrap();
                context.regs.zero();
                context.stack_frame.instruction_pointer = start as usize;
                // TODO remove magic numbers, kernel code segment
                context.stack_frame.code_segment = 0b1000;
                // TODO remove magic numbers, Interrupts enabled | reserved
                context.stack_frame.cpu_flags = 0x202;
                context.stack_frame.stack_pointer = stack_pointer;
                context.stack_frame.stack_segment = 0;
            }
            (context_pointer as *const Context).as_ref().unwrap()
        };

        Ok(KThread {
            id: ID.fetch_add(1, Ordering::Relaxed),
            stack: stack,
            context: Some(context),
            quanta: TICKS,
            state: State::Ready,
        })
    }
    /// Return the current "main" thread.
    ///
    /// # Safety
    /// This function may only be called once on the main thread
    pub unsafe fn main() -> KThread {
        assert_has_not_been_called!("The main kthread can be created only once!");
        let top = &kstack_late_bottom as *const _ as usize;
        let bottom = &kstack_top as *const _ as usize;
        KThread {
            id: ID.fetch_add(1, Ordering::Relaxed),
            stack: Stack::new(bottom, top),
            context: None, /* current thread */
            quanta: TICKS,
            state: State::Running,
        }
    }

    /// Return the "idle" thread
    ///
    /// # Safety
    /// Only call once
    ///
    /// # Side effects
    /// Allocates a global stack
    pub unsafe fn idle() -> KThread {
        assert_has_not_been_called!("The idle kthread can be created only once!");
        Self::new(idle).unwrap()
    }

    /// Put `context` into the given thread and return the context
    /// from the other thread. This should be used to swap threads.
    pub fn swap(&mut self, context: &'static Context, other: &mut KThread)
        -> &'static Context
    {
        assert!(self.context.is_none());
        self.context = Some(context);
        self.state = State::Ready;
        // give `other` the default time slice
        other.state = State::Running;
        other.quanta = TICKS;
        other.context.take().unwrap()
    }
}

extern "C" fn idle() {
    loop {
        unsafe { asm!("hlt") };
    }
}

pub extern "C" fn exit() -> ! {
    unsafe { asm!("int $0" :: "i"(EXIT_INT) :: "volatile") };
    unreachable!();
}
