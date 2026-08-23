//! Window-command application on [`super::WinitShell`] — the frameless
//! chrome controls (close/min/max, drag move, drag resize) drained from
//! the `sigil-events` queue by the host frame loop.
//!
//! Split out of `shell.rs` to keep the lifecycle / Shell-impl portions
//! readable under the 400-LoC hygiene cap.

use sigil_hal::{ResizeDirection, ShellEvent, WindowCommand, WindowEvent as SigilWindowEvent};

use crate::WinitWindowHandle;

use super::WinitShell;

impl WinitShell {
    /// Apply a [`WindowCommand`] to a live window.
    pub(crate) fn run_window_command(&mut self, handle: WinitWindowHandle, command: WindowCommand) {
        // Close replays the OS close-button path exactly: the app sees a
        // `CloseRequested` event, the loop exits after the next redraw.
        if command == WindowCommand::Close {
            self.push_event(ShellEvent::Window(SigilWindowEvent::CloseRequested));
            self.request_exit();
            self.request_redraws();
            return;
        }
        // A command naming a window that has not materialised yet is
        // dropped — there is nothing to minimise, move, or resize.
        let Some(window) = self.window_for(handle) else {
            return;
        };
        match command {
            WindowCommand::Minimize => window.set_minimized(true),
            WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
            // Drag failures are routine (button already released, or the
            // platform lacks the gesture — macOS has no drag-resize), so
            // the result is discarded rather than surfaced.
            WindowCommand::Drag => {
                let _ = window.drag_window();
            }
            WindowCommand::DragResize(direction) => {
                let _ = window.drag_resize_window(winit_direction(direction));
            }
            WindowCommand::Close => {}
        }
    }
}

const fn winit_direction(direction: ResizeDirection) -> winit::window::ResizeDirection {
    match direction {
        ResizeDirection::East => winit::window::ResizeDirection::East,
        ResizeDirection::North => winit::window::ResizeDirection::North,
        ResizeDirection::NorthEast => winit::window::ResizeDirection::NorthEast,
        ResizeDirection::NorthWest => winit::window::ResizeDirection::NorthWest,
        ResizeDirection::South => winit::window::ResizeDirection::South,
        ResizeDirection::SouthEast => winit::window::ResizeDirection::SouthEast,
        ResizeDirection::SouthWest => winit::window::ResizeDirection::SouthWest,
        ResizeDirection::West => winit::window::ResizeDirection::West,
    }
}
