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
            WindowCommand::Minimize => window.set_minimized(true),
            WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
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
