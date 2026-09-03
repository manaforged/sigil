use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use sigil_hal::Shell as _;
use winit::{event_loop::ActiveEventLoop, window::Window};

use super::{WinitShell, WinitWaker};
use crate::WinitWindowHandle;

pub(crate) struct ActionSender {
    tx: crossbeam_channel::Sender<ActionRequest>,
}

impl ActionHandler for ActionSender {
    fn do_action(&mut self, request: ActionRequest) {
        let _ = self.tx.send(request);
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

impl WinitShell {
    pub(crate) fn install_a11y_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        handle: WinitWindowHandle,
        window: &Window,
    ) {
        let action_sender = ActionSender {
            tx: self.a11y_action_tx.clone(),
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
        self.a11y_adapters.push((handle, adapter, activation));
    }

    pub(crate) fn process_a11y_event(
        &mut self,
        window_id: winit::window::WindowId,
        event: &winit::event::WindowEvent,
    ) {
        let Some(handle) = self
            .windows()
            .iter()
            .find_map(|(h, w)| (w.id() == window_id).then_some(*h))
        else {
            return;
        };
        let Some(window) = self.window_for(handle).cloned() else {
            return;
        };
        if let Some((_, adapter, _)) = self.a11y_adapters.iter_mut().find(|(h, _, _)| *h == handle)
        {
            adapter.process_event(&window, event);
        }
    }

    #[must_use]
    pub fn a11y_active(&self) -> bool {
        self.a11y_adapters.iter().any(|(_, _, a)| a.is_active())
    }

    #[must_use]
    pub fn take_a11y_activation(&self) -> bool {
        self.a11y_adapters
            .iter()
            .fold(false, |acc, (_, _, a)| a.take_pending() || acc)
    }

    pub fn publish_a11y_update(&mut self, update: &TreeUpdate) {
        for (_, adapter, _) in &mut self.a11y_adapters {
            let for_adapter = update.clone();
            adapter.update_if_active(|| for_adapter);
        }
    }

    #[must_use]
    pub fn a11y_action_receiver(&self) -> crossbeam_channel::Receiver<ActionRequest> {
        self.a11y_action_rx.clone()
    }
}

#[cfg(test)]
#[path = "a11y_tests.rs"]
mod tests;
