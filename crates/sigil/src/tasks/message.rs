use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

use tokio::sync::mpsc::{Receiver, UnboundedReceiver};

use crate::reactive::{Signal, signal};
use crate::tasks::current::scope_parts;
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::runtime::attach_task;

pub trait MessageReceiver<T>: Send + 'static {
    fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_;
}

impl<T: Send + 'static> MessageReceiver<T> for UnboundedReceiver<T> {
    fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_ {
        Self::recv(self)
    }
}

impl<T: Send + 'static> MessageReceiver<T> for Receiver<T> {
    fn recv(&mut self) -> impl Future<Output = Option<T>> + Send + '_ {
        Self::recv(self)
    }
}

type Close = Box<dyn FnOnce()>;

struct Handlers<T> {
    message: Box<dyn Fn(T)>,
    close: Cell<Option<Close>>,
}

#[track_caller]
pub fn on_message<T, F>(rx: impl MessageReceiver<T>, f: F)
where
    T: Send + 'static,
    F: Fn(T) + 'static,
{
    on_message_or_close(rx, f, || {});
}

#[track_caller]
pub fn on_message_or_close<T, F, C>(mut rx: impl MessageReceiver<T>, f: F, on_close: C)
where
    T: Send + 'static,
    F: Fn(T) + 'static,
    C: FnOnce() + 'static,
{
    let Some((owner, parts)) = scope_parts("on_message") else {
        return;
    };
    let handlers: Signal<Rc<Handlers<T>>> = signal(Rc::new(Handlers {
        message: Box::new(f),
        close: Cell::new(Some(Box::new(on_close))),
    }));
    let slot = handlers.into_sendable();
    let dispatcher = parts.dispatcher.clone();
    let work = async move {
        while let Some(value) = rx.recv().await {
            dispatch_to_owner(&dispatcher, owner, move || {
                if let Some(current) = slot.into_signal().try_peek() {
                    (current.message)(value);
                }
            });
        }
        dispatch_to_owner(&dispatcher, owner, move || {
            if let Some(close) = slot.into_signal().try_peek().and_then(|h| h.close.take()) {
                close();
            }
        });
    };
    let task = parts.backend.spawn(owner.id(), work, drop);
    attach_task(&parts, owner, task);
}
