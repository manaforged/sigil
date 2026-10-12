#![allow(clippy::redundant_pub_crate)]

use std::sync::atomic::Ordering;

use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent as WinitWindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

use crate::events::{ShellEvent, WindowEvent as SigilWindowEvent};

use crate::shell::{WinitWindowHandle, shell::WinitShell, waker::WakeEvent};

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

impl<F> WinitApp<F>
where
    F: FnMut(&mut WinitShell),
{
    fn run_frame(&mut self, event_loop: &ActiveEventLoop, target: Option<WinitWindowHandle>) {
        self.shell.set_redraw_target(target);
        (self.frame)(&mut self.shell);
        self.shell.set_redraw_target(None);
        if self.shell.exit_requested() {
            event_loop.exit();
        }
    }

    fn run_hidden(&mut self, event_loop: &ActiveEventLoop) {
        if !self.shell.any_visible() {
            self.run_frame(event_loop, None);
        }
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
        self.shell.request_redraws();
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: WakeEvent) {
        if !self.materialise_pending(event_loop) {
            return;
        }

        self.shell.pending_wake().store(false, Ordering::Release);

        self.app_redraw_pending = true;
        self.shell.request_redraws();
        self.run_hidden(event_loop);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.app_redraw_pending = true;
            self.shell.request_redraws();
            self.run_hidden(event_loop);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        crate::shell::exit_hook::run();
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
                if let Some(handle) = self.shell.handle_for(window_id) {
                    self.shell
                        .push_event(handle, ShellEvent::Window(SigilWindowEvent::CloseRequested));
                }
                self.app_redraw_pending = true;
                self.shell.request_redraws();
                self.run_hidden(event_loop);
            }
            WinitWindowEvent::RedrawRequested => {
                if !self.materialise_pending(event_loop) {
                    return;
                }
                if !self.app_redraw_pending {
                    self.shell.set_force_paint();
                }
                self.app_redraw_pending = false;
                let target = self.shell.handle_for(window_id);
                self.run_frame(event_loop, target);
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
