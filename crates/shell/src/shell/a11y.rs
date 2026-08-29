use std::sync::{Arc, Mutex};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use winit::{event_loop::ActiveEventLoop, window::Window};

use super::WinitShell;
use crate::WinitWindowHandle;

type LatestTree = Arc<Mutex<Option<TreeUpdate>>>;

pub(crate) struct ActionSender {
    tx: crossbeam_channel::Sender<ActionRequest>,
}

impl ActionHandler for ActionSender {
    fn do_action(&mut self, request: ActionRequest) {
        let _ = self.tx.send(request);
    }
}

struct LatestTreeActivationHandler {
    latest: LatestTree,
}

impl ActivationHandler for LatestTreeActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

struct NoopDeactivationHandler;

impl DeactivationHandler for NoopDeactivationHandler {
    fn deactivate_accessibility(&mut self) {}
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
        let latest: LatestTree = Arc::new(Mutex::new(None));
        let activation_handler = LatestTreeActivationHandler {
            latest: Arc::clone(&latest),
        };
        let adapter = accesskit_winit::Adapter::with_direct_handlers(
            event_loop,
            window,
            activation_handler,
            action_sender,
            NoopDeactivationHandler,
        );
        window.set_visible(true);
        self.a11y_adapters.push((handle, adapter, latest));
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

    pub fn publish_a11y_update(&mut self, update: &TreeUpdate) {
        for (_, adapter, latest) in &mut self.a11y_adapters {
            *latest
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(update.clone());
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
