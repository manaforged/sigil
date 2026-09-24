use crate::hal::{ResizeDirection, ShellEvent, WindowCommand, WindowEvent as SigilWindowEvent};

use crate::shell::WinitWindowHandle;

use super::WinitShell;

impl WinitShell {
    pub(crate) fn run_window_command(&mut self, handle: WinitWindowHandle, command: WindowCommand) {
        if command == WindowCommand::Close {
            self.push_event(ShellEvent::Window(SigilWindowEvent::CloseRequested));
            self.request_exit();
            self.request_redraws();
            return;
        }
        let Some(window) = self.window_for(handle) else {
            return;
        };
        match command {
            WindowCommand::SetZoom(percent) if percent > 0 => {
                let id = window.id();
                let factor = window.scale_factor();
                let size = window.inner_size();
                let zoom = f32::from(percent) / 100.0;
                if let Some((_, stored)) = self
                    .window_zooms
                    .iter_mut()
                    .find(|(stored, _)| *stored == handle)
                {
                    *stored = zoom;
                }
                self.push_scale_factor_changed(id, factor);
                self.push_event(ShellEvent::Window(SigilWindowEvent::Resized(
                    crate::hal::PhysicalSize::new(size.width, size.height),
                )));
                self.request_redraws();
            }
            WindowCommand::SetZoom(_) | WindowCommand::Close => {}
            WindowCommand::Minimize => window.set_minimized(true),
            WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
            WindowCommand::Drag => {
                if let Err(err) = window.drag_window() {
                    tracing::warn!(target: "sigil::shell", error = %err, "window drag failed");
                }
            }
            WindowCommand::DragResize(direction) => {
                if let Err(err) = window.drag_resize_window(winit_direction(direction)) {
                    tracing::warn!(target: "sigil::shell", error = %err, "window drag-resize failed");
                }
            }
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
