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
use interrupts::{Context, SLEEP_INT};

use self::thread::{KThread, State, TICKS};

mod thread;

// the scheduler must be created at runtime, but will not be called until
// interrupts are enabled, which must be after the scheduler is created.
/// The sole `Scheduler` instance
static SCHEDULER: IrqLock<Option<Scheduler>> = IrqLock::new(None);

/// Basic round-robin scheduler
struct Scheduler {
    // State::Ready
    threads: VecDeque<KThread>,
    // State::Sleeping -- delta queue
    sleeping: VecDeque<KThread>,
    // None => current == idle
    current: Option<KThread>,
    idle: KThread,
}

impl Scheduler {
    fn new() -> Scheduler {
        unsafe { Scheduler {
            threads: VecDeque::new(),
            sleeping: VecDeque::new(),
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
/// If there are no available threads then the idle thread will be run.
pub fn sched_yield(current_stack: &'static Context) -> &'static Context {
    let mut lock = SCHEDULER.lock();
    let &mut Scheduler {
        ref mut threads,
        ref mut sleeping,
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

/// Make the current thread sleep for `time` quanta
///
/// The thread is not gurenteed to run after `time` is complete, it will simply
/// be resumed.
/// `time` must not be zero
pub fn sched_sleep(current_stack: &'static Context, time: u8) -> &'static Context {
    let mut lock = SCHEDULER.lock();
    let &mut Scheduler {
        ref mut threads,
        ref mut sleeping,
        ref mut current,
        ref mut idle,
    } = lock.as_mut().unwrap();

    // first, swap out with a new thread
    let mut current_thread = current.take().unwrap();
    let mut next_thread = threads.pop_front();

    let ret = {
        let next = next_thread.as_mut().unwrap_or(idle);
        current_thread.swap(current_stack, next)
    };
    *current = next_thread;

    // now put it in the sleeping list
    current_thread.state = State::Sleeping;
    current_thread.quanta = time;

    // calculate index for the current thread in the delta queue
    // Also calcuate the delta from the previous item
    let index = sleeping.iter().take_while(|elem| {
            match elem.quanta {
                x if x <= current_thread.quanta => {
                    current_thread.quanta -= elem.quanta;
                    true
                },
                _ => false,
            }
        }).count();
    // first, update the delta for the element following, if it exists
    if let Some(next) = sleeping.get_mut(index) {
        next.quanta -= current_thread.quanta;
    }
    // now lets put it in the queue
    sleeping.insert(index, current_thread);

    ret
}

/// Reduce the current thread's time slice by one tick. If it has no
/// time left then yield to a new thread.
pub fn tick(current_stack: &'static Context) -> &'static Context {
    let mut lock = SCHEDULER.lock();
    let &mut Scheduler {
        ref mut threads,
        ref mut sleeping,
        ref mut current,
        ref mut idle,
    } = lock.as_mut().unwrap();

    // update the sleeping thread list
    if let Some(thread) = sleeping.front_mut() {
        thread.quanta -=1;
    }
    loop {
        let should_pop = sleeping.front()
            .map_or(false, |thread| thread.quanta == 0);
        if should_pop {
            threads.push_back(sleeping.pop_front().unwrap());
        } else {
            break;
        }
    }

    // now update the running thread
    {
        let mut running = current.as_mut().unwrap_or(idle);

        running.quanta -= 1;
        if running.quanta > 0 {
            // continue with the current thread
            return current_stack;
        }
    }
    let mut next_thread = threads.pop_front();

    if next_thread.is_none() && current.is_none() {
        // Only the idle thread can run
        idle.quanta = TICKS;
        return current_stack;
    }

    // Now swap threads
    let ret = {
        if let Some(current_thread) = current {
            // `next_thread` is unknown and current is running
            let next = next_thread.as_mut().unwrap_or(idle);
            current_thread.swap(current_stack, next)
        } else {
            // `next_thread` must be Some(_) and idle is running
            idle.swap(current_stack, next_thread.as_mut().unwrap())
        }
    };

    if let Some(current) = current.take() {
        threads.push_back(current);
    }

    *current = next_thread;
    ret
}

/// Reschedule the current kernel thread
pub fn thread_yield() {
    unsafe {
        asm!("mov rax, 0
              int $0"
              :: "i"(SLEEP_INT)
              : "rax"
              : "intel", "volatile")
    }
}

pub fn thread_sleep(time: u8) {
    unsafe {
        asm!("movzx rax, $1
              int $0"
              :: "i"(SLEEP_INT),"r"(time)
              : "rax"
              : "intel", "volatile")
    }
}

pub fn test() {
    add(A);
    add(B);
    thread_sleep(255);
    println!("Re-entered the main thread!");
}

extern "C" fn A() {
    loop {
        println!("A");
        thread_sleep(20)
    }
}
extern "C" fn B() {
    loop {
        println!("\tB");
        thread_sleep(20)
    }
}

/// Tests
#[cfg(feature = "test")]
pub mod tests {
    use tap::TestGroup;
    pub fn run() {
        test_yield();
    }

    fn test_yield() {
        let mut tap = TestGroup::new(1);
        tap.diagnostic("Testing yield");
        super::add(new);
        super::thread_yield();
        tap.ok(Some("Thread Returned"));
    }

    extern "C" fn new() {
        super::thread_yield();
    }
}
