use std::cell::Cell;
use std::rc::Rc;

use super::WidgetLayoutExt;
use crate::events::{
    FileDropEvent, GestureEvent, ImeEvent, KeyEvent, NodeEventsExt, PasteEvent, PointerEvent,
    ScrollEvent, Shortcut,
};
use crate::kernel::NodeBuilder;
use crate::layout::CaretGeometry;
use crate::reactive::Signal;

macro_rules! forward_handlers {
    ($($name:ident($($arg:ty),*);)*) => {$(
        fn $name<H: Fn($($arg),*) + 'static>(self, h: H) -> Self {
            self.map_node(move |node| <NodeBuilder as NodeEventsExt>::$name(node, h))
        }
    )*};
}

macro_rules! forward_shortcuts {
    ($($name:ident;)*) => {$(
        fn $name<S, H>(self, shortcut: S, h: H) -> Self
        where
            S: Into<Shortcut>,
            H: Fn(&mut KeyEvent) + 'static,
        {
            let shortcut = shortcut.into();
            self.map_node(move |node| <NodeBuilder as NodeEventsExt>::$name(node, shortcut, h))
        }
    )*};
}

impl<T: WidgetLayoutExt> NodeEventsExt for T {
    forward_handlers! {
        pointer_down(&mut PointerEvent);
        pointer_down_capture(&mut PointerEvent);
        pointer_up(&mut PointerEvent);
        pointer_up_capture(&mut PointerEvent);
        pointer_move(&mut PointerEvent);
        pointer_move_capture(&mut PointerEvent);
        pointer_enter(&mut PointerEvent);
        pointer_leave(&mut PointerEvent);
        file_drop(&FileDropEvent);
        link(&str);
        on_selection_copy(&str, &crate::AppResult<crate::clipboard::Delivery>);
        on_scroll(&mut ScrollEvent);
        key_any(&mut KeyEvent);
        key_capture_any(&mut KeyEvent);
        on_ime(&ImeEvent);
        on_gesture(&mut GestureEvent);
        on_paste(&mut PasteEvent);
        focus_in();
        focus_out();
    }

    forward_shortcuts! {
        key;
        shortcut;
        key_capture;
    }

    fn ime_caret(self, anchor: Rc<Cell<Option<CaretGeometry>>>) -> Self {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::ime_caret(node, anchor))
    }

    fn ime_eligible<F>(self, f: F) -> Self
    where
        F: Fn() -> bool + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::ime_eligible(node, f))
    }

    fn selectable_text(self, yes: bool) -> Self {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::selectable_text(node, yes))
    }

    fn autofocus(self) -> Self {
        self.map_node(<NodeBuilder as NodeEventsExt>::autofocus)
    }

    fn focus_when(self, trigger: Signal<u64>) -> Self {
        self.map_node(move |node| node.focus_when(trigger))
    }
}
