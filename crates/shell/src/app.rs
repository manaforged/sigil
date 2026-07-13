//! winit `ApplicationHandler` implementation for the sigil shell.
//!
//! The handler is the single point that drives `WinitShell` from inside
//! a running event loop: it materialises pending windows on `resumed`,
//! runs the `sigil-async` drain hook on each user wake, buffers window
//! events through the shell, and invokes the frame callback on redraw.

#![allow(clippy::redundant_pub_crate)]

use std::sync::atomic::Ordering;

use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent as WinitWindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

use hal::{ShellEvent, WindowEvent as SigilWindowEvent};

use crate::{shell::WinitShell, waker::WakeEvent};

pub(crate) struct WinitApp<F> {
    pub(crate) shell: WinitShell,
    pub(crate) frame: F,
    /// True when we issued `request_redraws()` to coalesce a buffered input event,
    /// so the resulting `RedrawRequested` is ours and the change-gate may skip its
    /// paint if the input changed nothing. Cleared on each `RedrawRequested`; when
    /// it is already clear, that redraw is OS-driven (first-show / un-occlude /
    /// resize) and must force a paint — otherwise the drawable shows gray.
    pub(crate) input_redraw_pending: bool,
}

impl<F> ApplicationHandler<WakeEvent> for WinitApp<F>
where
    F: FnMut(&mut WinitShell),
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.shell.materialise_pending(event_loop);
        for (_, window) in self.shell.windows() {
            window.request_redraw();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: WakeEvent) {
        // Materialise any windows requested between ticks so that
        // frame-time `create_window` calls resolve on the next event
        // loop iteration rather than hanging forever.
        self.shell.materialise_pending(event_loop);

        if let Some(hook) = self.shell.wake_hook() {
            hook();
        }

        // Release pairs with the Acquire on the waker's
        // compare_exchange. After this store, subsequent `wake()` calls
        // can schedule a fresh delivery — but the hook has already
        // observed any writes that happened-before those calls.
        self.shell.pending_wake().store(false, Ordering::Release);

        // Wake hook drained the MPSC; trigger a repaint so any
        // signal-driven updates land in the next frame.
        for (_, window) in self.shell.windows() {
            window.request_redraw();
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // A `WaitUntil` deadline elapsed — a `cx.interval` (via
        // `on_idle_deadline` → `next_interval_deadline`) has come due.
        // winit does not raise `RedrawRequested` on its own for a timed
        // wake, so request one here; the ensuing frame runs
        // `tick_intervals`, firing the due callbacks. Only
        // `ResumeTimeReached` triggers this — `Poll` / `WaitCancelled`
        // must not, or the loop would busy-spin.
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.shell.request_redraws();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Apply the configured idle deadline (e.g. `cx.interval` wakeups)
        // so the OS can wake the loop on time without busy-spinning.
        // Falls back to `Wait` when no deadline hook is installed.
        match self.shell.deadline_hook().and_then(|hook| hook()) {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        // AccessKit's own contract: `process_event` must see every window
        // event before the application handles it (DR-050 §1).
        self.shell.process_a11y_event(window_id, &event);
        match event {
            WinitWindowEvent::CloseRequested => {
                self.shell
                    .push_event(ShellEvent::Window(SigilWindowEvent::CloseRequested));
                self.shell.request_exit();
                self.shell.request_redraws();
            }
            WinitWindowEvent::RedrawRequested => {
                // Drain any late-arriving window specs so frame-time
                // creation calls complete before the frame callback
                // runs.
                self.shell.materialise_pending(event_loop);
                // A redraw we did NOT schedule for input coalescing is OS-driven
                // (first-show / un-occlude / resize): the drawable needs pixels, so
                // force the change-gate to paint this frame. Our own input-coalescing
                // redraws leave the gate free to skip a no-op paint.
                if !self.input_redraw_pending {
                    self.shell.set_force_paint();
                }
                self.input_redraw_pending = false;
                (self.frame)(&mut self.shell);
                if self.shell.exit_requested() {
                    event_loop.exit();
                }
            }
            event => {
                if self.shell.buffer_window_event(window_id, event) {
                    self.input_redraw_pending = true;
                    self.shell.request_redraws();
                }
            }
        }
    }
}
