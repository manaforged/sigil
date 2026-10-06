use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::hal::Shell as _;
use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use winit::{event_loop::ActiveEventLoop, window::Window};

use super::WinitShell;
use crate::shell::WinitWaker;
use crate::shell::WinitWindowHandle;

pub(crate) struct ActionSender {
    tx: crossbeam_channel::Sender<ActionRequest>,
    waker: WinitWaker,
}

impl ActionHandler for ActionSender {
    fn do_action(&mut self, request: ActionRequest) {
        let _ = self.tx.send(request);
        let _ = self.waker.wake();
    }
}

#[derive(Default)]
pub(crate) struct Activation {
    active: AtomicBool,
    pending: AtomicBool,
}

impl Activation {
    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn take_pending(&self) -> bool {
        self.pending.swap(false, Ordering::AcqRel)
    }

    fn activate(&self) {
        self.active.store(true, Ordering::Release);
        self.pending.store(true, Ordering::Release);
    }

    fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
        self.pending.store(false, Ordering::Release);
    }
}

struct ActivationSwitch {
    activation: Arc<Activation>,
    waker: WinitWaker,
}

impl ActivationHandler for ActivationSwitch {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.activation.activate();
        let _ = self.waker.wake();
        None
    }
}

impl DeactivationHandler for ActivationSwitch {
    fn deactivate_accessibility(&mut self) {
        self.activation.deactivate();
    }
}

pub(crate) struct AdapterEntry {
    pub(crate) handle: WinitWindowHandle,
    adapter: accesskit_winit::Adapter,
    activation: Arc<Activation>,
    actions: crossbeam_channel::Receiver<ActionRequest>,
}

impl WinitShell {
    pub(crate) fn install_a11y_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        handle: WinitWindowHandle,
        window: &Window,
    ) {
        let (tx, actions) = crossbeam_channel::unbounded();
        let action_sender = ActionSender {
            tx,
            waker: self.waker(),
        };
        let activation = Arc::new(Activation::default());
        let switch = || ActivationSwitch {
            activation: Arc::clone(&activation),
            waker: self.waker(),
        };
        let adapter = accesskit_winit::Adapter::with_direct_handlers(
            event_loop,
            window,
            switch(),
            action_sender,
            switch(),
        );
        window.set_visible(true);
        self.a11y_adapters.push(AdapterEntry {
            handle,
            adapter,
            activation,
            actions,
        });
    }

    pub(crate) fn process_a11y_event(
        &mut self,
        window_id: winit::window::WindowId,
        event: &winit::event::WindowEvent,
    ) {
        let Some(handle) = self.handle_for(window_id) else {
            return;
        };
        let Some(window) = self.window_for(handle).cloned() else {
            return;
        };
        if let Some(entry) = self.a11y_adapters.iter_mut().find(|e| e.handle == handle) {
            entry.adapter.process_event(&window, event);
        }
    }

    fn adapter(&self, handle: WinitWindowHandle) -> Option<&AdapterEntry> {
        self.a11y_adapters.iter().find(|e| e.handle == handle)
    }

    #[must_use]
    pub fn a11y_active(&self, handle: WinitWindowHandle) -> bool {
        self.adapter(handle)
            .is_some_and(|e| e.activation.is_active())
    }

    #[must_use]
    pub fn take_a11y_activation(&self, handle: WinitWindowHandle) -> bool {
        self.adapter(handle)
            .is_some_and(|e| e.activation.take_pending())
    }

    pub fn publish_a11y_update(&mut self, handle: WinitWindowHandle, update: TreeUpdate) {
        if let Some(entry) = self.a11y_adapters.iter_mut().find(|e| e.handle == handle) {
            entry.adapter.update_if_active(|| update);
        }
    }

    #[must_use]
    pub fn a11y_action_receiver(
        &self,
        handle: WinitWindowHandle,
    ) -> Option<crossbeam_channel::Receiver<ActionRequest>> {
        self.adapter(handle).map(|e| e.actions.clone())
    }
}

#[cfg(test)]
#[path = "a11y_tests.rs"]
mod tests;
