use crate::hal::{FileDropEvent, KeyEvent, PointerEvent, PointerPhase, ScrollEvent, Shortcut};

type PointerHandler = Box<dyn Fn(&mut PointerEvent) + 'static>;
type KeyHandler = Box<dyn Fn(&mut KeyEvent) + 'static>;
type ScrollHandler = Box<dyn Fn(&mut ScrollEvent) + 'static>;
type FocusHandler = Box<dyn Fn() + 'static>;
type FileDropHandler = Box<dyn Fn(&FileDropEvent) + 'static>;

struct KeyBinding {
    shortcut: Shortcut,
    handler: KeyHandler,
}

#[derive(Default)]
pub struct EventHandlers {
    pointer_down_capture: Vec<PointerHandler>,
    pointer_down: Vec<PointerHandler>,
    pointer_up_capture: Vec<PointerHandler>,
    pointer_up: Vec<PointerHandler>,
    pointer_move_capture: Vec<PointerHandler>,
    pointer_move: Vec<PointerHandler>,
    pointer_enter: Vec<PointerHandler>,
    pointer_leave: Vec<PointerHandler>,
    scroll: Vec<ScrollHandler>,
    key_capture: Vec<KeyBinding>,
    key: Vec<KeyBinding>,
    key_any: Vec<KeyHandler>,
    focus_in: Vec<FocusHandler>,
    focus_out: Vec<FocusHandler>,
    file_drop: Vec<FileDropHandler>,
}

impl EventHandlers {
    pub(crate) fn add_pointer_down_capture(&mut self, handler: PointerHandler) {
        self.pointer_down_capture.push(handler);
    }

    pub(crate) fn add_pointer_down(&mut self, handler: PointerHandler) {
        self.pointer_down.push(handler);
    }

    pub(crate) fn add_pointer_up_capture(&mut self, handler: PointerHandler) {
        self.pointer_up_capture.push(handler);
    }

    pub(crate) fn add_pointer_up(&mut self, handler: PointerHandler) {
        self.pointer_up.push(handler);
    }

    pub(crate) fn add_pointer_move_capture(&mut self, handler: PointerHandler) {
        self.pointer_move_capture.push(handler);
    }

    pub(crate) fn add_pointer_move(&mut self, handler: PointerHandler) {
        self.pointer_move.push(handler);
    }

    pub(crate) fn add_pointer_enter(&mut self, handler: PointerHandler) {
        self.pointer_enter.push(handler);
    }

    pub(crate) fn add_pointer_leave(&mut self, handler: PointerHandler) {
        self.pointer_leave.push(handler);
    }

    pub(crate) fn add_scroll(&mut self, handler: ScrollHandler) {
        self.scroll.push(handler);
    }
    pub(crate) fn add_file_drop(&mut self, handler: FileDropHandler) {
        self.file_drop.push(handler);
    }
    pub(crate) fn invoke_file_drop(&self, event: &FileDropEvent) -> bool {
        for handler in &self.file_drop {
            handler(event);
        }
        !self.file_drop.is_empty()
    }

    pub(crate) fn invoke_scroll(&self, event: &mut ScrollEvent) -> bool {
        let mut invoked = false;
        for handler in &self.scroll {
            handler(event);
            invoked = true;
            if event.propagation_stopped() {
                return invoked;
            }
        }
        invoked
    }

    pub(crate) fn add_key_capture(&mut self, shortcut: Shortcut, handler: KeyHandler) {
        self.key_capture.push(KeyBinding { shortcut, handler });
    }

    pub(crate) fn add_key(&mut self, shortcut: Shortcut, handler: KeyHandler) {
        self.key.push(KeyBinding { shortcut, handler });
    }

    pub(crate) fn add_key_any(&mut self, handler: KeyHandler) {
        self.key_any.push(handler);
    }

    pub(crate) fn add_focus_in(&mut self, handler: FocusHandler) {
        self.focus_in.push(handler);
    }

    pub(crate) fn add_focus_out(&mut self, handler: FocusHandler) {
        self.focus_out.push(handler);
    }

    pub(crate) fn invoke_pointer(
        &self,
        phase: PointerPhase,
        capture: bool,
        event: &mut PointerEvent,
    ) {
        let Some(handlers) = self.pointer_handlers(phase, capture) else {
            return;
        };
        for handler in handlers {
            handler(event);
            if event.propagation_stopped() {
                return;
            }
        }
    }

    pub(crate) fn invoke_key(&self, event: &mut KeyEvent, capture: bool) -> bool {
        let handlers = if capture {
            &self.key_capture
        } else {
            &self.key
        };
        let mut invoked = false;
        for binding in handlers {
            if !binding.shortcut.matches(event) {
                continue;
            }
            (binding.handler)(event);
            invoked = true;
            if event.propagation_stopped() {
                return invoked;
            }
        }
        if capture {
            return invoked;
        }
        for handler in &self.key_any {
            handler(event);
            invoked = true;
            if event.propagation_stopped() {
                return invoked;
            }
        }
        invoked
    }

    pub(crate) fn invoke_focus_in(&self) {
        for handler in &self.focus_in {
            handler();
        }
    }

    pub(crate) fn invoke_focus_out(&self) {
        for handler in &self.focus_out {
            handler();
        }
    }

    fn pointer_handlers(&self, phase: PointerPhase, capture: bool) -> Option<&[PointerHandler]> {
        match (phase, capture) {
            (PointerPhase::Down, true) => Some(&self.pointer_down_capture),
            (PointerPhase::Down, false) => Some(&self.pointer_down),
            (PointerPhase::Up, true) => Some(&self.pointer_up_capture),
            (PointerPhase::Up, false) => Some(&self.pointer_up),
            (PointerPhase::Move, true) => Some(&self.pointer_move_capture),
            (PointerPhase::Move, false) => Some(&self.pointer_move),
            (PointerPhase::Enter, false) => Some(&self.pointer_enter),
            (PointerPhase::Leave, false) => Some(&self.pointer_leave),
            _ => None,
        }
    }
}

impl core::fmt::Debug for EventHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EventHandlers").finish_non_exhaustive()
    }
}
