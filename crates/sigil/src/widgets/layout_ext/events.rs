use std::cell::Cell;
use std::rc::Rc;

use super::WidgetLayoutExt;
use crate::events::{
    FileDropEvent, ImeEvent, KeyEvent, NodeEventsExt, PointerEvent, ScrollEvent, Shortcut,
};
use crate::kernel::NodeBuilder;
use crate::layout::CaretGeometry;
use crate::reactive::Signal;

impl<T: WidgetLayoutExt> NodeEventsExt for T {
    fn pointer_down<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_down(node, h))
    }

    fn pointer_down_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_down_capture(node, h))
    }

    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_up(node, h))
    }

    fn pointer_up_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_up_capture(node, h))
    }

    fn pointer_move<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_move(node, h))
    }

    fn pointer_move_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_move_capture(node, h))
    }

    fn pointer_enter<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_enter(node, h))
    }

    fn pointer_leave<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::pointer_leave(node, h))
    }

    fn file_drop<H>(self, h: H) -> Self
    where
        H: Fn(&FileDropEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::file_drop(node, h))
    }

    fn link<H>(self, h: H) -> Self
    where
        H: Fn(&str) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::link(node, h))
    }

    fn on_scroll<H>(self, h: H) -> Self
    where
        H: Fn(&mut ScrollEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::on_scroll(node, h))
    }

    fn key_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::key_any(node, h))
    }

    fn key_capture_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::key_capture_any(node, h))
    }

    fn on_ime<H>(self, h: H) -> Self
    where
        H: Fn(&ImeEvent) + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::on_ime(node, h))
    }

    fn key<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::key(node, shortcut, h))
    }

    fn shortcut<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::shortcut(node, shortcut, h))
    }

    fn key_capture<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::key_capture(node, shortcut, h))
    }

    fn focus_in<H>(self, h: H) -> Self
    where
        H: Fn() + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::focus_in(node, h))
    }

    fn focus_out<H>(self, h: H) -> Self
    where
        H: Fn() + 'static,
    {
        self.map_node(move |node| <NodeBuilder as NodeEventsExt>::focus_out(node, h))
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
