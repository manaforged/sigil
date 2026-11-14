use winit::window::CursorIcon;

use crate::events::Cursor;
use crate::shell::WinitWindowHandle;

use super::WinitShell;
use super::commands::winit_direction;

impl WinitShell {
    pub(super) fn show_cursor(&mut self, handle: WinitWindowHandle, cursor: Cursor) {
        let shown = self
            .window_cursors
            .iter()
            .find_map(|(h, shown)| (*h == handle).then_some(*shown))
            .unwrap_or_default();
        if shown == cursor {
            return;
        }
        let Some(window) = self.window_for(handle) else {
            return;
        };
        window.set_cursor(winit_cursor(cursor));
        match self.window_cursors.iter_mut().find(|(h, _)| *h == handle) {
            Some((_, stored)) => *stored = cursor,
            None => self.window_cursors.push((handle, cursor)),
        }
    }
}

fn winit_cursor(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Default => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::Text => CursorIcon::Text,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::Grabbing => CursorIcon::Grabbing,
        Cursor::Crosshair => CursorIcon::Crosshair,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::Move => CursorIcon::Move,
        Cursor::Wait => CursorIcon::Wait,
        Cursor::Resize(direction) => CursorIcon::from(winit_direction(direction)),
        Cursor::ColumnResize => CursorIcon::ColResize,
        Cursor::RowResize => CursorIcon::RowResize,
    }
}
