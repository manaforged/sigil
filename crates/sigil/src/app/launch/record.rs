use crate::app::config::AppConfig;
use crate::app::embed::Host;
use crate::app::error::{AppResult, Error};
use crate::app::window::{
    CloseAction, Mount, WindowId, WindowSignals, register_window, set_current_window,
    unregister_window,
};
use crate::events::{EventRouter, ShellEvent, WindowEvent};
use crate::reactive::Signal;
use crate::shell::WinitWindowHandle;

use super::backend::Surface;
use super::scene_cache::SceneCache;

pub(crate) struct WindowView {
    pub(crate) id: WindowId,
    pub(crate) handle: WinitWindowHandle,
    pub(crate) config: AppConfig,
    pub(crate) surface: Option<Surface>,
    pub(crate) viewport: (f32, f32),
    pub(crate) scene_cache: SceneCache,
    pub(crate) inbox: Vec<ShellEvent>,
    pub(crate) signals: WindowSignals,
    pub(crate) hidden: bool,
    pub(crate) repaint: bool,
    pub(crate) painted: bool,
}

pub(crate) struct WindowRecord {
    pub(crate) host: Host,
    pub(crate) router: EventRouter,
    pub(crate) view: WindowView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Routed {
    Delivered,
    Close,
    Destroyed,
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseOutcome {
    Removed { last: bool },
    Hidden,
    Kept,
}

impl WindowRecord {
    pub(crate) fn new(
        id: WindowId,
        handle: WinitWindowHandle,
        mut config: AppConfig,
        mount: Mount,
    ) -> AppResult<Self> {
        if config.user_zoom <= 0.0 {
            config.user_zoom = 1.0;
        }
        let signals = register_window(id);
        let mut host = Host::new();
        let previous = set_current_window(Some(id));
        mount(&mut host);
        set_current_window(previous);
        if host.root().is_none() {
            return Err(Error::new(std::io::Error::other(
                "the mounted tree has no root node",
            )));
        }
        let logical = super::logical_size_from_config(config.size);
        let viewport = (
            logical.width / config.user_zoom,
            logical.height / config.user_zoom,
        );
        host.layout(viewport).map_err(Error::from)?;
        let view = WindowView {
            id,
            handle,
            config,
            surface: None,
            viewport,
            scene_cache: SceneCache::default(),
            inbox: Vec::new(),
            signals,
            hidden: false,
            repaint: false,
            painted: false,
        };
        Ok(Self {
            host,
            router: EventRouter::new(),
            view,
        })
    }
}

pub(crate) type SignalWrite = (Signal<bool>, bool);

#[derive(Default)]
pub(crate) struct Windows {
    records: Vec<WindowRecord>,
    signal_writes: Vec<SignalWrite>,
}

impl Windows {
    pub(crate) fn insert(&mut self, record: WindowRecord) {
        self.records.push(record);
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub(crate) fn get_mut(&mut self, handle: WinitWindowHandle) -> Option<&mut WindowRecord> {
        self.records.iter_mut().find(|r| r.view.handle == handle)
    }

    #[cfg(feature = "inspect")]
    pub(crate) fn first_mut(&mut self) -> Option<&mut WindowRecord> {
        self.records.first_mut()
    }

    #[must_use]
    pub(crate) fn first_handle(&self) -> Option<WinitWindowHandle> {
        self.records.first().map(|r| r.view.handle)
    }

    #[must_use]
    pub(crate) fn handle_of(&self, id: WindowId) -> Option<WinitWindowHandle> {
        self.records
            .iter()
            .find_map(|r| (r.view.id == id).then_some(r.view.handle))
    }

    pub(crate) fn route(&mut self, handle: WinitWindowHandle, event: ShellEvent) -> Routed {
        let Some(record) = self.get_mut(handle) else {
            return Routed::Dropped;
        };
        let write = match &event {
            ShellEvent::Window(WindowEvent::CloseRequested) => return Routed::Close,
            ShellEvent::Window(WindowEvent::Destroyed) => return Routed::Destroyed,
            ShellEvent::Window(WindowEvent::Focused(focused)) => {
                Some((record.view.signals.focused, *focused))
            }
            ShellEvent::Window(WindowEvent::Occluded(occluded)) => {
                Some((record.view.signals.occluded, *occluded))
            }
            _ => None,
        };
        record.view.inbox.push(event);
        self.signal_writes.extend(write);
        Routed::Delivered
    }

    pub(crate) fn close_requested(&mut self, handle: WinitWindowHandle) -> CloseOutcome {
        let Some(record) = self.get_mut(handle) else {
            return CloseOutcome::Kept;
        };
        match record.view.config.on_close.decide() {
            CloseAction::Close => self.remove(handle),
            CloseAction::Hide => {
                record.view.hidden = true;
                CloseOutcome::Hidden
            }
            CloseAction::Keep => CloseOutcome::Kept,
        }
    }

    pub(crate) fn remove(&mut self, handle: WinitWindowHandle) -> CloseOutcome {
        let Some(index) = self.records.iter().position(|r| r.view.handle == handle) else {
            return CloseOutcome::Kept;
        };
        unregister_window(self.records.remove(index).view.id);
        CloseOutcome::Removed {
            last: self.records.is_empty(),
        }
    }

    pub(crate) fn take_signal_writes(&mut self) -> Vec<SignalWrite> {
        core::mem::take(&mut self.signal_writes)
    }

    pub(crate) fn mark_others_for_repaint(&mut self, painted: WinitWindowHandle) {
        for record in &mut self.records {
            if record.view.handle != painted {
                record.view.repaint = true;
            }
        }
    }

    pub(crate) fn set_hidden(&mut self, handle: WinitWindowHandle, hidden: bool) {
        if let Some(record) = self.get_mut(handle) {
            record.view.hidden = hidden;
        }
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
