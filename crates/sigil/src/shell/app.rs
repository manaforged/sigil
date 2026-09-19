#![allow(clippy::redundant_pub_crate)]

use std::sync::atomic::Ordering;

use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent as WinitWindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

use crate::hal::{ShellEvent, WindowEvent as SigilWindowEvent};

use crate::shell::{shell::WinitShell, waker::WakeEvent};

pub(crate) struct WinitApp<F> {
    pub(crate) shell: WinitShell,
    pub(crate) frame: F,
    pub(crate) app_redraw_pending: bool,
}

impl<F> WinitApp<F> {
    fn materialise_pending(&mut self, event_loop: &ActiveEventLoop) -> bool {
        self.shell.materialise_pending(event_loop);
        if self.shell.has_failed_windows() {
            event_loop.exit();
            return false;
        }
        true
    }
}

impl<F> ApplicationHandler<WakeEvent> for WinitApp<F>
where
    F: FnMut(&mut WinitShell),
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.materialise_pending(event_loop) {
            return;
        }
        for (_, window) in self.shell.windows() {
            window.request_redraw();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: WakeEvent) {
        if !self.materialise_pending(event_loop) {
            return;
        }

        if let Some(hook) = self.shell.wake_hook() {
            hook();
        }

        self.shell.pending_wake().store(false, Ordering::Release);

        self.app_redraw_pending = true;
        for (_, window) in self.shell.windows() {
            window.request_redraw();
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.app_redraw_pending = true;
            self.shell.request_redraws();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
        self.shell.process_a11y_event(window_id, &event);
        match event {
            WinitWindowEvent::CloseRequested => {
                self.shell
                    .push_event(ShellEvent::Window(SigilWindowEvent::CloseRequested));
                self.shell.request_exit();
                self.shell.request_redraws();
            }
            WinitWindowEvent::RedrawRequested => {
                if !self.materialise_pending(event_loop) {
                    return;
                }
                if !self.app_redraw_pending {
                    self.shell.set_force_paint();
                }
                self.app_redraw_pending = false;
                (self.frame)(&mut self.shell);
                if self.shell.exit_requested() {
                    event_loop.exit();
                }
            }
            event => {
                if self.shell.buffer_window_event(window_id, event) {
                    self.app_redraw_pending = true;
                    self.shell.request_redraws();
                }
            }
        }
    }
}
