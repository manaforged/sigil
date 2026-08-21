//! AccessKit winit-adapter wiring (DR-050 §1, W5-A11Y).
//!
//! `accesskit_winit::Adapter::with_direct_handlers` requires its three
//! handler closures to be `Send` — sigil's `Cx`/`EventRouter` are
//! deliberately `!Send`, so the handlers here cannot reach into the
//! reactive runtime directly. Instead, [`ActionSender`] is the only
//! `Send` surface: it forwards every inbound `ActionRequest` across a
//! `crossbeam-channel` to the UI thread, where
//! [`super::WinitShell::drain_a11y_actions`] (called from the app's
//! frame loop) picks it up and routes it through `EventRouter`
//! (`sigil_hal::A11yBridge`'s doc comment at `crates/hal/src/a11y.rs:44-48`
//! anticipated exactly this shape).

use std::sync::{Arc, Mutex};

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};
use winit::{event_loop::ActiveEventLoop, window::Window};

use super::WinitShell;
use crate::WinitWindowHandle;

/// Latest full `TreeUpdate` this window's adapter has seen, shared
/// between the `!Send` publish path and the `Send`-bound
/// `ActivationHandler` (RC-focus adversarial finding 2).
type LatestTree = Arc<Mutex<Option<TreeUpdate>>>;

/// `Send` action-request forwarder installed as the adapter's
/// `ActionHandler`. Runs on whatever thread the platform AT-SPI/UIA/AX
/// backend delivers on; the only thing it does is push onto the
/// channel, so it never touches the `!Send` reactive runtime.
pub(crate) struct ActionSender {
    tx: crossbeam_channel::Sender<ActionRequest>,
}

impl ActionHandler for ActionSender {
    fn do_action(&mut self, request: ActionRequest) {
        // Channel is unbounded and the UI thread drains every frame; a
        // closed receiver only happens after the shell has dropped, at
        // which point silently discarding the action is correct.
        let _ = self.tx.send(request);
    }
}

/// Returns the latest published tree, if any. `request_initial_tree` is
/// the ONLY synchronous return path AccessKit gives an adapter that
/// activates late (e.g. a screen reader attaches mid-session): without
/// this, an AT that activates after startup with no subsequent UI
/// mutation would never see a tree — `update_if_active` only forwards
/// updates published WHILE active, and nothing here marks the runtime
/// dirty to force a fresh publish. `latest` is updated on every
/// `publish_a11y_update` call regardless of activation state, so
/// whatever's cached here is always the real current tree, not a
/// placeholder.
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

/// No-op `DeactivationHandler` — sigil keeps publishing every frame
/// regardless of AT presence; there is no per-window state to tear
/// down when the platform adapter deactivates.
struct NoopDeactivationHandler;

impl DeactivationHandler for NoopDeactivationHandler {
    fn deactivate_accessibility(&mut self) {}
}

impl WinitShell {
    /// Construct the AccessKit adapter for a freshly created window and
    /// make the window visible.
    ///
    /// Must run before the window has ever been shown —
    /// `accesskit_winit::Adapter::with_direct_handlers` panics
    /// otherwise — so callers pass the just-created (still-hidden)
    /// window and this method flips visibility on afterward.
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

    /// Forward one winit window event to every matching AccessKit
    /// adapter. Must run before the event reaches any other handling —
    /// `accesskit_winit::Adapter::process_event`'s own contract.
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

    /// Publish a freshly projected `TreeUpdate` to every live window's
    /// adapter (today there is exactly one window; broadcasting is
    /// correct rather than picking a handle if that ever changes).
    /// Updates each adapter's `LatestTree` cache unconditionally — RC-focus
    /// finding 2 — so a late-activating AT sees the real current tree via
    /// `LatestTreeActivationHandler` instead of nothing.
    pub fn publish_a11y_update(&mut self, update: &TreeUpdate) {
        for (_, adapter, latest) in &mut self.a11y_adapters {
            *latest
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(update.clone());
            let for_adapter = update.clone();
            adapter.update_if_active(|| for_adapter);
        }
    }

    /// Clone of the inbound `ActionRequest` receiver. Cheap (an `Arc`
    /// clone under the hood) — callers hold their own clone for the
    /// lifetime of the frame loop rather than borrowing `&self` each
    /// frame.
    #[must_use]
    pub fn a11y_action_receiver(&self) -> crossbeam_channel::Receiver<ActionRequest> {
        self.a11y_action_rx.clone()
    }
}

#[cfg(test)]
#[path = "a11y_tests.rs"]
mod tests;
