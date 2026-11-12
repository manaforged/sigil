use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

use tokio::sync::mpsc::{Receiver, UnboundedReceiver};

use crate::reactive::{Signal, signal};
use crate::tasks::current::scope_parts;
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::runtime::attach_task;

/// A channel receiver that `tasks::listen` reads.
pub trait MessageReceiver<T>: Send + 'static {
    /// Waits for the next message; returns `None` when the channel closes.
    fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_;

    /// Takes a message that is already waiting, without waiting for one.
    fn try_recv(&mut self) -> Option<T> {
        None
    }
}

impl<T: Send + 'static> MessageReceiver<T> for UnboundedReceiver<T> {
    fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_ {
        Self::recv(self)
    }

    fn try_recv(&mut self) -> Option<T> {
        Self::try_recv(self).ok()
    }
}

impl<T: Send + 'static> MessageReceiver<T> for Receiver<T> {
    fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_ {
        Self::recv(self)
    }

    fn try_recv(&mut self) -> Option<T> {
        Self::try_recv(self).ok()
    }
}

/// The most waiting messages handled in one batch, so a flood still yields to frames.
const BURST: usize = 256;

type Close = Box<dyn FnOnce()>;

struct Handlers<T> {
    message: Box<dyn Fn(T)>,
    close: Cell<Option<Close>>,
}

/// Runs `f` on the UI thread per message, then `close` when all senders drop.
///
/// Messages already waiting are handled together in one reactive batch, so effects run once per burst.
#[track_caller]
pub fn listen<T, F, C>(mut rx: impl MessageReceiver<T>, f: F, close: C)
where
    T: Send + 'static,
    F: Fn(T) + 'static,
    C: FnOnce() + 'static,
{
    let location = core::panic::Location::caller();
    let Some((owner, parts)) = scope_parts("listen") else {
        return;
    };
    let handlers: Signal<Rc<Handlers<T>>> = signal(Rc::new(Handlers {
        message: Box::new(f),
        close: Cell::new(Some(Box::new(close))),
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
            if let Some(hook) = slot.into_signal().try_peek().and_then(|h| h.close.take()) {
                hook();
            }
        });
    };
    let task = parts.backend.spawn(owner, work, drop);
    attach_task(&parts, owner, task);
}
