use crate::events::CursorIcon;
use crate::shell::WinitWindowHandle;

use super::WinitShell;

impl WinitShell {
    pub(super) fn show_cursor(&mut self, handle: WinitWindowHandle, cursor: CursorIcon) {
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

const fn winit_cursor(cursor: CursorIcon) -> winit::window::CursorIcon {
    use winit::window::CursorIcon as Shown;
    match cursor {
        CursorIcon::Default => Shown::Default,
        CursorIcon::Pointer => Shown::Pointer,
        CursorIcon::Text => Shown::Text,
        CursorIcon::Grab => Shown::Grab,
        CursorIcon::Grabbing => Shown::Grabbing,
        CursorIcon::Crosshair => Shown::Crosshair,
        CursorIcon::NotAllowed => Shown::NotAllowed,
        CursorIcon::Move => Shown::Move,
        CursorIcon::Wait => Shown::Wait,
        CursorIcon::EResize => Shown::EResize,
        CursorIcon::NResize => Shown::NResize,
        CursorIcon::NeResize => Shown::NeResize,
        CursorIcon::NwResize => Shown::NwResize,
        CursorIcon::SResize => Shown::SResize,
        CursorIcon::SeResize => Shown::SeResize,
        CursorIcon::SwResize => Shown::SwResize,
        CursorIcon::WResize => Shown::WResize,
        CursorIcon::EwResize => Shown::EwResize,
        CursorIcon::NsResize => Shown::NsResize,
        CursorIcon::NeswResize => Shown::NeswResize,
        CursorIcon::NwseResize => Shown::NwseResize,
        CursorIcon::ColResize => Shown::ColResize,
        CursorIcon::RowResize => Shown::RowResize,
    }
}
