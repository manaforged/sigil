//! Slow work, off the frame thread.
//!
//! A terminal app reads files, runs subprocesses, and waits on networks. Doing
//! any of that between two frames stops the app: input queues, animations
//! stall, and the screen holds still until it returns.
//!
//! [`work`] runs a job on a worker thread and hands back a signal that fills
//! in when the job finishes. The frame loop collects finished jobs on the same
//! tick it drives animations and timers, so a result lands on a frame boundary
//! and the tree rebuilds from it like any other signal change.
//!
//! No async runtime is involved. A terminal binary that needs "re-scan every
//! few seconds without freezing" should not have to stand one up.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use sigil_reactive::{Signal, signal};
use sigil_tasks::Status;

/// A job whose result is still on its way.
///
/// Boxed so jobs of different result types share one queue; each carries the
/// closure that writes its own signal.
type Pending = Box<dyn FnMut() -> bool>;

thread_local! {
    static WAITING: RefCell<Vec<Pending>> = const { RefCell::new(Vec::new()) };
}

/// Run `job` on a worker thread. The signal reports [`Status::Pending`] until
/// it finishes, then what it returned.
///
/// The job must not touch the tree: signals belong to the frame thread. Hand
/// back a value and let the tree read it.
///
/// A job that panics reports [`Status::Err`] with its message rather than
/// waiting forever, and the frame loop carries on.
pub fn work<T, F>(job: F) -> Signal<Status<T, String>>
where
    T: Clone + Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let out = signal(Status::Pending);
    let (tx, rx): (_, Receiver<Result<T, String>>) = channel();
    std::thread::spawn(move || {
        let done = catch_unwind(AssertUnwindSafe(job)).map_err(|payload| blame(&*payload));
        // A closed channel means the frame loop stopped caring; drop quietly.
        drop(tx.send(done));
    });
    WAITING.with_borrow_mut(|waiting| {
        waiting.push(Box::new(move || match rx.try_recv() {
            Ok(Ok(value)) => {
                out.set_always(Status::Ok(value));
                true
            }
            Ok(Err(why)) => {
                out.set_always(Status::Err(why));
                true
            }
            Err(TryRecvError::Empty) => false,
            // The worker panicked or was dropped. Nothing to report.
            Err(TryRecvError::Disconnected) => true,
        }));
    });
    out
}

/// Drop every job this thread is waiting on.
pub fn abandon() {
    WAITING.with_borrow_mut(Vec::clear);
}

/// Write back every job that has finished. Returns whether any did.
///
/// The frame loop calls this each tick. A host driving [`crate::step`] itself
/// gets it for free; a test pumping its own loop calls it directly.
pub fn collect_work() -> bool {
    // Take the list: a signal write can start another job, and holding the
    // borrow across that would panic.
    let mut waiting = WAITING.with_borrow_mut(std::mem::take);
    let before = waiting.len();
    waiting.retain_mut(|job| !job());
    let landed = waiting.len() != before;
    WAITING.with_borrow_mut(|next| {
        waiting.append(next);
        *next = waiting;
    });
    landed
}

/// What a panicking job left behind.
fn blame(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| String::from("the job panicked"))
        },
        |text| (*text).to_owned(),
    )
}
