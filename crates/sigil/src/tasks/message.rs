use std::cell::Cell;
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

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(clippy::expect_used)]
mod tests {
    use super::sealed::Sealed;

    #[test]
    fn std_receiver_delivers_in_order_then_reports_close() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime");
        let (tx, mut rx) = std::sync::mpsc::channel();
        let sender = std::thread::spawn(move || {
            for n in 1..=3_u32 {
                tx.send(n).expect("receiver alive");
            }
        });
        let received = runtime.block_on(async {
            let mut received = Vec::new();
            while let Some(n) = Sealed::recv(&mut rx).await {
                received.push(n);
            }
            received
        });
        sender.join().expect("sender thread");
        assert_eq!(received, vec![1, 2, 3]);
        assert_eq!(Sealed::try_recv(&mut rx), None);
    }
}
