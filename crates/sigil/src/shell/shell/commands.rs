use std::sync::Arc;

use crate::app::{ResizeDirection, WindowCommand, WindowLevel};
use crate::events::{ShellEvent, WindowEvent as SigilWindowEvent};
use winit::{dpi::LogicalPosition as WinitLogicalPosition, window::Window};

use crate::shell::WinitWindowHandle;

use super::WinitShell;

impl WinitShell {
    pub(crate) fn run_window_command(&mut self, handle: WinitWindowHandle, command: WindowCommand) {
        if command == WindowCommand::Close {
            self.push_event(handle, ShellEvent::Window(SigilWindowEvent::CloseRequested));
            self.request_redraws();
            return;
        }
        let Some(window) = self.window_for(handle).cloned() else {
            return;
        };
        match command {
            WindowCommand::SetZoom(percent) if percent > 0 => {
                self.set_zoom(handle, &window, percent);
            }
            WindowCommand::SetZoom(_) | WindowCommand::Close => {}
            command => apply_to_window(&window, command),
        }
    }

    fn set_zoom(&mut self, handle: WinitWindowHandle, window: &Arc<Window>, percent: u16) {
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
        self.event_window = handle;
        self.push_scale_factor_changed(id, factor);
        self.push_event(
            handle,
            ShellEvent::Window(SigilWindowEvent::Resized(
                crate::geometry::PhysicalSize::new(size.width, size.height),
            )),
        );
        self.request_redraws();
    }
}

fn apply_to_window(window: &Window, command: WindowCommand) {
    match command {
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
        WindowCommand::Show => {
            window.set_visible(true);
            window.request_redraw();
        }
        WindowCommand::Hide => window.set_visible(false),
        WindowCommand::Focus => {
            window.set_visible(true);
            window.focus_window();
        }
        WindowCommand::SetTitle(title) => window.set_title(&title),
        WindowCommand::SetPosition(point) => window.set_outer_position(WinitLogicalPosition::new(
            f64::from(point.x),
            f64::from(point.y),
        )),
        WindowCommand::SetLevel(level) => window.set_window_level(winit_level(level)),
        WindowCommand::Close | WindowCommand::SetZoom(_) => {}
    }
}

const fn winit_level(level: WindowLevel) -> winit::window::WindowLevel {
    match level {
        WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
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
