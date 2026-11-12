#[cfg(any(feature = "menu", global_hotkey))]
use std::cell::Cell;
use std::cell::RefCell;
#[cfg(not(native_commands))]
use std::marker::PhantomData;

use std::sync::mpsc::{Receiver, Sender, channel};

use crate::app::Wake;
#[cfg(native_commands)]
use crate::reactive::EventChannel;

pub(crate) struct Native<Cmd, Handle, Ev: 'static> {
    #[cfg(native_commands)]
    pending: RefCell<Option<Cmd>>,
    #[cfg(not(native_commands))]
    _command: PhantomData<Cmd>,
    installed: RefCell<Option<Handle>>,
    #[cfg(native_commands)]
    events: EventChannel<Ev>,
    tx: Sender<Ev>,
    rx: Receiver<Ev>,
    #[cfg(any(feature = "menu", global_hotkey))]
    connected: Cell<bool>,
}

impl<Cmd, Handle, Ev: Clone + Send + 'static> Native<Cmd, Handle, Ev> {
    pub(crate) fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            #[cfg(native_commands)]
            pending: RefCell::new(None),
            #[cfg(not(native_commands))]
            _command: PhantomData,
            installed: RefCell::new(None),
            #[cfg(native_commands)]
            events: EventChannel::new(),
            tx,
            rx,
            #[cfg(any(feature = "menu", global_hotkey))]
            connected: Cell::new(false),
        }
    }

    #[cfg(native_commands)]
    pub(crate) fn queue(&self, command: Cmd) {
        *self.pending.borrow_mut() = Some(command);
    }

    #[cfg(feature = "menu")]
    pub(crate) fn has_pending(&self) -> bool {
        self.pending.borrow().is_some()
    }

    #[cfg(native_commands)]
    pub(crate) fn take(&self) -> Option<Cmd> {
        self.pending.borrow_mut().take()
    }

    #[cfg(feature = "menu")]
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

    #[cfg(any(feature = "tray", feature = "media", global_hotkey, test))]
    pub(crate) fn installed<R>(&self, visit: impl FnOnce(&mut Option<Handle>) -> R) -> R {
        visit(&mut self.installed.borrow_mut())
    }

    #[cfg(native_commands)]
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

    #[cfg(any(feature = "menu", global_hotkey))]
    pub(crate) fn sink_once(
        &self,
        wake: &dyn Fn() -> Wake,
    ) -> Option<impl Fn(Ev) + Send + Sync + use<Cmd, Handle, Ev>> {
        (!self.connected.replace(true)).then(|| self.sink(wake()))
    }

    #[cfg(native_commands)]
    pub(crate) fn pump(&self) {
        self.drain().for_each(|value| self.events.emit(value));
    }

    pub(crate) fn drain(&self) -> impl Iterator<Item = Ev> + '_ {
        self.rx.try_iter()
    }
}
