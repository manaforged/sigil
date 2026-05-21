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
    event::WindowEvent as WinitWindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

use hal::{ShellEvent, WindowEvent as SigilWindowEvent};

use crate::{shell::WinitShell, waker::WakeEvent};

pub(crate) struct WinitApp<F> {
    pub(crate) shell: WinitShell,
    pub(crate) frame: F,
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
                (self.frame)(&mut self.shell);
                if self.shell.exit_requested() {
                    event_loop.exit();
                }
            }
            event => {
                if self.shell.buffer_window_event(window_id, event) {
                    self.shell.request_redraws();
                }
            }
        }
    }
}
