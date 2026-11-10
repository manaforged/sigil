use crossbeam_channel::{Receiver, Sender, unbounded};

#[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
use crate::reactive::EventChannel;

pub(crate) use crate::app::Wake;

pub(crate) struct Bridge<T> {
    tx: Sender<T>,
    rx: Receiver<T>,
}

impl<T: Send + 'static> Bridge<T> {
    pub(crate) fn new() -> Self {
        let (tx, rx) = unbounded();
        Self { tx, rx }
    }

    pub(crate) fn sink(&self, wake: Wake) -> impl Fn(T) + Send + Sync + use<T> {
        let tx = self.tx.clone();
        move |value| {
            drop(tx.send(value));
            wake();
        }
    }

    pub(crate) fn drain(&self) -> impl Iterator<Item = T> + '_ {
        self.rx.try_iter()
    }

    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    pub(crate) fn forward(&self, channel: &EventChannel<T>)
    where
        T: Clone,
    {
        self.drain().for_each(|value| channel.emit(value));
    }
}
