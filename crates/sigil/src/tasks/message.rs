use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use crate::reactive::{Signal, signal};
use crate::tasks::current::scope_parts;
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::runtime::attach_task;

pub(crate) mod sealed {
    use std::future::Future;

    use tokio::sync::mpsc::{Receiver, UnboundedReceiver};

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed<T>: Send + 'static {
        fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_;

        fn try_recv(&mut self) -> Option<T> {
            None
        }
    }

    impl<T: Send + 'static> Sealed<T> for UnboundedReceiver<T> {
        fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_ {
            Self::recv(self)
        }

        fn try_recv(&mut self) -> Option<T> {
            Self::try_recv(self).ok()
        }
    }

    impl<T: Send + 'static> Sealed<T> for Receiver<T> {
        fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_ {
            Self::recv(self)
        }

        fn try_recv(&mut self) -> Option<T> {
            Self::try_recv(self).ok()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl<T: Send + 'static> Sealed<T> for std::sync::mpsc::Receiver<T> {
        async fn recv(&mut self) -> Option<T> {
            let rx = std::mem::replace(self, std::sync::mpsc::channel().1);
            let (rx, message) = crate::tasks::blocking(move || {
                let message = rx.recv().ok();
                (rx, message)
            })
            .await;
            *self = rx;
            message
        }

        fn try_recv(&mut self) -> Option<T> {
            Self::try_recv(self).ok()
        }
    }
}

/// A channel receiver that `tasks::listen` reads: a tokio `mpsc` receiver or a `std::sync::mpsc::Receiver`.
pub trait MessageReceiver<T>: sealed::Sealed<T> + Send + 'static {}

impl<T: Send + 'static> MessageReceiver<T> for tokio::sync::mpsc::Receiver<T> {}

impl<T: Send + 'static> MessageReceiver<T> for tokio::sync::mpsc::UnboundedReceiver<T> {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + 'static> MessageReceiver<T> for std::sync::mpsc::Receiver<T> {}

/// The most waiting messages handled in one batch, so a flood still yields to frames.
const BURST: usize = 256;

enum Close {
    Open,
    Waiting(Box<dyn FnOnce()>),
    Ended,
}

fn end(close: &Cell<Close>) {
    if let Close::Waiting(hook) = close.replace(Close::Ended) {
        hook();
    }
}

struct Handlers<T> {
    message: Box<dyn Fn(T)>,
    close: Rc<Cell<Close>>,
}

/// Handle from `tasks::listen` that sets what runs once after every sender drops; dropping it keeps the listening.
pub struct Listener {
    close: Rc<Cell<Close>>,
}

impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listener").finish_non_exhaustive()
    }
}

impl Listener {
    /// Runs `on_close` on the UI thread once after the last message, or now when the channel has already closed.
    pub fn closed(self, on_close: impl FnOnce() + 'static) {
        let hook: Box<dyn FnOnce()> = Box::new(on_close);
        match self.close.replace(Close::Ended) {
            Close::Ended => hook(),
            Close::Open | Close::Waiting(_) => self.close.set(Close::Waiting(hook)),
        }
    }
}

/// Runs `on_message` on the UI thread for each message, batching waiting ones, until every sender drops or the calling scope ends.
#[allow(
    clippy::must_use_candidate,
    reason = "listening runs without the handle; it only sets the close hook"
)]
#[track_caller]
pub fn listen<T: Send + 'static>(
    mut rx: impl MessageReceiver<T>,
    on_message: impl Fn(T) + 'static,
) -> Listener {
    let location = core::panic::Location::caller();
    let close = Rc::new(Cell::new(Close::Open));
    let listener = Listener {
        close: Rc::clone(&close),
    };
    let Some((owner, parts)) = scope_parts("listen") else {
        return listener;
    };
    let handlers: Signal<Rc<Handlers<T>>> = signal(Rc::new(Handlers {
        message: Box::new(on_message),
        close,
    }));
    let slot = handlers.into_sendable();
    let dispatcher = parts.dispatcher.clone();
    let work = async move {
        while let Some(first) = rx.recv().await {
            let mut burst = vec![first];
            while burst.len() < BURST
                && let Some(next) = rx.try_recv()
            {
                burst.push(next);
            }
            dispatch_to_owner(&dispatcher, owner, move || {
                if let Some(current) = slot.into_signal().try_peek() {
                    let _site = crate::profile::site(location, crate::profile::SiteKind::Message);
                    crate::reactive::batch(|| {
                        for value in burst {
                            (current.message)(value);
                        }
                    });
                }
            });
        }
        dispatch_to_owner(&dispatcher, owner, move || {
            if let Some(current) = slot.into_signal().try_peek() {
                end(&current.close);
            }
        });
    };
    let task = parts.backend.spawn(owner, work, drop);
    attach_task(&parts, owner, task);
    listener
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(clippy::expect_used)]
#[path = "message_tests.rs"]
mod tests;
