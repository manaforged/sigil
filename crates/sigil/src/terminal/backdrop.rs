use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::app::backdrop::{from_dark_light, set_system_backdrop, system_backdrop};
use crate::reactive::{Owner, Signal, signal};
use crate::tasks::dispatch::dispatch_to_owner;
use crate::terminal::Backdrop;

const POLL: Duration = Duration::from_millis(250);

thread_local! {
    static BACKDROP: Cell<Option<Signal<Option<Backdrop>>>> = const { Cell::new(None) };
}

/// The terminal background brightness, updated when the OS light or dark setting changes.
#[must_use]
pub(crate) fn backdrop() -> Signal<Option<Backdrop>> {
    BACKDROP.with(|slot| {
        slot.get().unwrap_or_else(|| {
            let value = Owner::new("sigil::backdrop", None).with(|| signal(None));
            slot.set(Some(value));
            value
        })
    })
}

pub(crate) struct Follow {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    owner: Owner,
}

impl Follow {
    pub(crate) fn start(probed: Option<Backdrop>) -> Self {
        let system = system_backdrop().peek();
        backdrop().set(probed.or(system));
        let stop = Arc::new(AtomicBool::new(false));
        let owner = Owner::new("sigil::backdrop_follow", None);
        let follows = system.is_some() && (probed.is_none() || probed == system);
        let thread = follows
            .then(crate::tasks::current::dispatcher)
            .flatten()
            .and_then(|dispatcher| {
                let watcher = dark_light::subscribe().ok()?;
                let stop = Arc::clone(&stop);
                Some(std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        if let Some(next) = watcher.try_iter().filter_map(from_dark_light).last() {
                            dispatch_to_owner(&dispatcher, owner, move || {
                                backdrop().set(Some(next));
                                set_system_backdrop(next);
                            });
                        }
                        std::thread::park_timeout(POLL);
                    }
                }))
            });
        Self {
            stop,
            thread,
            owner,
        }
    }
}

impl Drop for Follow {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            drop(thread.join());
        }
        self.owner.dispose();
    }
}
