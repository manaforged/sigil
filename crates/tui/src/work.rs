use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use sigil::reactive::{Signal, on_cleanup, signal};
use sigil::tasks::Status;

type Pending = Box<dyn FnMut() -> bool>;

thread_local! {
    static WAITING: RefCell<Vec<Pending>> = const { RefCell::new(Vec::new()) };
}

pub fn work<T, F>(job: F) -> Signal<Status<T, String>>
where
    T: Clone + Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let out = signal(Status::Pending);
    let alive = Rc::new(Cell::new(true));
    let cleanup = Rc::clone(&alive);
    on_cleanup(move || cleanup.set(false));
    let (tx, rx): (_, Receiver<Result<T, String>>) = channel();
    std::thread::spawn(move || {
        let done = catch_unwind(AssertUnwindSafe(job)).map_err(|payload| blame(&*payload));
        drop(tx.send(done));
    });
    WAITING.with_borrow_mut(|waiting| {
        waiting.push(Box::new(move || match rx.try_recv() {
            Ok(Ok(value)) => {
                if alive.get() {
                    out.set_always(Status::Ok(value));
                }
                true
            }
            Ok(Err(why)) => {
                if alive.get() {
                    out.set_always(Status::Err(why));
                }
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => true,
        }));
    });
    out
}

pub fn abandon() {
    WAITING.with_borrow_mut(Vec::clear);
}

pub fn collect_work() -> bool {
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
