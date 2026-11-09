#[cfg(feature = "menu")]
use std::cell::Cell;
use std::cell::RefCell;
#[cfg(not(any(feature = "menu", feature = "tray", feature = "media")))]
use std::marker::PhantomData;

use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::app::Wake;
#[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
use crate::reactive::EventChannel;

pub(crate) struct Native<Cmd, Handle, Ev: 'static> {
    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    pending: RefCell<Option<Cmd>>,
    #[cfg(not(any(feature = "menu", feature = "tray", feature = "media")))]
    _command: PhantomData<Cmd>,
    installed: RefCell<Option<Handle>>,
    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    events: EventChannel<Ev>,
    tx: Sender<Ev>,
    rx: Receiver<Ev>,
    #[cfg(feature = "menu")]
    connected: Cell<bool>,
}

impl<Cmd, Handle, Ev: Clone + Send + 'static> Native<Cmd, Handle, Ev> {
    pub(crate) fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
            pending: RefCell::new(None),
            #[cfg(not(any(feature = "menu", feature = "tray", feature = "media")))]
            _command: PhantomData,
            installed: RefCell::new(None),
            #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
            events: EventChannel::new(),
            tx,
            rx,
            #[cfg(feature = "menu")]
            connected: Cell::new(false),
        }
    }

    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    pub(crate) fn queue(&self, command: Cmd) {
        *self.pending.borrow_mut() = Some(command);
    }

    #[cfg(any(feature = "menu", feature = "tray"))]
    pub(crate) fn has_pending(&self) -> bool {
        self.pending.borrow().is_some()
    }

    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    pub(crate) fn take(&self) -> Option<Cmd> {
        self.pending.borrow_mut().take()
    }

    #[cfg(any(feature = "menu", feature = "tray"))]
    pub(crate) fn install<E>(
        &self,
        build: impl FnOnce(Cmd) -> Result<Handle, E>,
    ) -> Result<bool, E> {
        let Some(command) = self.take() else {
            return Ok(false);
        };
        let handle = build(command)?;
        *self.installed.borrow_mut() = Some(handle);
        Ok(true)
    }

    #[cfg(any(
        feature = "tray",
        feature = "media",
        all(feature = "hotkey", not(target_arch = "wasm32")),
        test
    ))]
    pub(crate) fn installed<R>(&self, visit: impl FnOnce(&mut Option<Handle>) -> R) -> R {
        visit(&mut self.installed.borrow_mut())
    }

    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    pub(crate) fn events(&self) -> EventChannel<Ev> {
        self.events.clone()
    }

    pub(crate) fn sink(&self, wake: Wake) -> impl Fn(Ev) + Send + Sync + use<Cmd, Handle, Ev> {
        let tx = self.tx.clone();
        move |value| {
            drop(tx.send(value));
            wake();
        }
    }

    #[cfg(feature = "menu")]
    pub(crate) fn sink_once(
        &self,
        wake: Wake,
    ) -> Option<impl Fn(Ev) + Send + Sync + use<Cmd, Handle, Ev>> {
        (!self.connected.replace(true)).then(|| self.sink(wake))
    }

    #[cfg(any(feature = "menu", feature = "tray", feature = "media"))]
    pub(crate) fn pump(&self) {
        self.drain().for_each(|value| self.events.emit(value));
    }

    pub(crate) fn drain(&self) -> impl Iterator<Item = Ev> + '_ {
        self.rx.try_iter()
    }
}
