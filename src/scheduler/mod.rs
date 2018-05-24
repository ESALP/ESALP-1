// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.
#![allow(unused)]

use alloc::vec_deque::VecDeque;

use sync::IrqLock;
use interrupts::{Context,YIELD_INT};

use self::thread::KThread;

mod thread;

// the scheduler must be created at runtime, but will not be called until
// interrupts are enabled, which must be after the scheduler is created.
/// The sole `Scheduler` instance
static SCHEDULER: IrqLock<Option<Scheduler>> = IrqLock::new(None);

/// Basic round-robin scheduler
struct Scheduler {
    threads: VecDeque<KThread>,
    // None => current == idle
    current: Option<KThread>,
    idle: KThread,
}

impl Scheduler {
    fn new() -> Scheduler {
        unsafe { Scheduler {
            threads: VecDeque::new(),
            current: Some(KThread::main()),
            idle: KThread::idle(),
        }}
    }
}

/// Initialize the scheduler structure. May only be called once.
pub fn init() {
    *SCHEDULER.lock() = Some(Scheduler::new());
}

/// Create a new thread that will start with the `start` function
pub fn add(start: extern "C" fn()) -> Result<(), &'static str>{
    let thread = KThread::new(start)?;

    let mut lock = SCHEDULER.lock();
    let sched = lock.as_mut().unwrap();
    sched.threads.push_back(thread);
    Ok(())
}

/// Yield the thread that `current_stack` belongs to to a new thread.
///
/// If there are no available threads then the idle thread will be
/// run.
pub fn sched_yield(current_stack: &'static Context) -> &'static Context {
    let mut lock = SCHEDULER.lock();
    let &mut Scheduler {
        ref mut threads,
        ref mut current,
        ref mut idle,
    } = lock.as_mut().unwrap();

    let mut current_thread = current.take().unwrap();
    let mut next_thread = threads.pop_front();

    let ret = {
        let next = next_thread.as_mut().unwrap_or(idle);
        current_thread.swap(current_stack, next)
    };

    threads.push_back(current_thread);

    *current = next_thread;

    ret
}

/// Reschedule the current kernel thread
pub fn thread_yield() {
    unsafe {
        asm!("int $0" :: "i"(YIELD_INT) :: "volatile")
    }
}

/// Tests
#[cfg(feature = "test")]
pub mod tests {
    use tap::TestGroup;
    pub fn run(tap: &mut TestGroup) {
        tap.diagnostic("Testing threading");
        test_yield(tap);
    }

    fn test_yield(tap: &mut TestGroup) {
        super::add(new);
        super::thread_yield();
        tap.ok(Some("Thread Returned"));
    }

    extern "C" fn new() {
        super::thread_yield();
    }
}
