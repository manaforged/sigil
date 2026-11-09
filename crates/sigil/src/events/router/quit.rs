use crate::app::config::QuitKeys;
use crate::events::{KeyEvent, KeyPhase};
use crate::kernel::Cx;
use crate::layout::LayoutEngine;

use super::EventRouter;

impl EventRouter {
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub(crate) const fn set_quit(&mut self, quit: QuitKeys) {
        self.quit = Some(quit);
    }

    pub(super) fn route_quit_key(
        &mut self,
        cx: &mut Cx,
        layout: &LayoutEngine,
        event: KeyEvent,
    ) -> bool {
        let quits = self.quit.is_some_and(|keys| pressed(keys, &event));
        let handled = self.route_key(cx, layout, event);
        if quits && !handled {
            crate::app::exit::request_exit();
        }
        handled
    }
}

fn pressed(keys: QuitKeys, event: &KeyEvent) -> bool {
    event.phase == KeyPhase::Down
        && !event.repeat
        && ((keys.esc && event.is_escape())
            || (keys.ctrl_c && event.modifiers.ctrl() && event.is_letter_c()))
}
