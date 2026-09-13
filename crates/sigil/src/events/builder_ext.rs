use std::cell::Cell;
use std::rc::Rc;

use crate::hal::{FileDropEvent, ImeEvent, KeyEvent, PointerEvent, ScrollEvent, Shortcut};
use crate::kernel::{Node, NodeBuilder};
use crate::layout::CaretGeometry;
use accesskit::Action;

use crate::events::EventHandlers;
use crate::events::ime_handlers::{CaretAnchorExt, ImeHandlers};

pub trait NodeEventsExt {
    #[must_use]
    fn pointer_down<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_down_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_up_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_move<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_move_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_enter<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn pointer_leave<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn file_drop<H>(self, h: H) -> Self
    where
        H: Fn(&FileDropEvent) + 'static;

    #[must_use]
    fn on_scroll<H>(self, h: H) -> Self
    where
        H: Fn(&mut ScrollEvent) + 'static;

    #[must_use]
    fn key<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static;

    #[must_use]
    fn key_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static;

    #[must_use]
    fn key_capture<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static;

    #[must_use]
    fn focus_in<H>(self, h: H) -> Self
    where
        H: Fn() + 'static;

    #[must_use]
    fn focus_out<H>(self, h: H) -> Self
    where
        H: Fn() + 'static;

    #[must_use]
    fn on_ime<H>(self, h: H) -> Self
    where
        H: Fn(&ImeEvent) + 'static;

    #[must_use]
    fn ime_caret(self, anchor: Rc<Cell<Option<CaretGeometry>>>) -> Self;

    #[must_use]
    fn ime_eligible<F>(self, f: F) -> Self
    where
        F: Fn() -> bool + 'static;
}

impl NodeEventsExt for NodeBuilder {
    fn pointer_down<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_pointer_down(Box::new(h)))
    }

    fn pointer_down_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_down_capture(Box::new(h));
        })
    }

    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self.action(Action::Click), move |handlers| {
            handlers.add_pointer_up(Box::new(h));
        })
    }

    fn pointer_up_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_up_capture(Box::new(h));
        })
    }

    fn pointer_move<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_pointer_move(Box::new(h)))
    }

    fn pointer_move_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_move_capture(Box::new(h));
        })
    }

    fn pointer_enter<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_enter(Box::new(h));
        })
    }

    fn file_drop<H>(self, h: H) -> Self
    where
        H: Fn(&FileDropEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_file_drop(Box::new(h));
        })
    }

    fn pointer_leave<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_leave(Box::new(h));
        })
    }

    fn on_scroll<H>(self, h: H) -> Self
    where
        H: Fn(&mut ScrollEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_scroll(Box::new(h));
        })
    }

    fn key<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        with_handlers(self, move |handlers| {
            handlers.add_key(shortcut, Box::new(h));
        })
    }

    fn key_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_key_any(Box::new(h)))
    }

    fn key_capture<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        with_handlers(self, move |handlers| {
            handlers.add_key_capture(shortcut, Box::new(h));
        })
    }

    fn focus_in<H>(self, h: H) -> Self
    where
        H: Fn() + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_focus_in(Box::new(h)))
    }

    fn focus_out<H>(self, h: H) -> Self
    where
        H: Fn() + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_focus_out(Box::new(h)))
    }

    fn on_ime<H>(self, h: H) -> Self
    where
        H: Fn(&ImeEvent) + 'static,
    {
        with_ime_handlers(self, move |handlers| handlers.add(Box::new(h)))
    }

    fn ime_caret(self, anchor: Rc<Cell<Option<CaretGeometry>>>) -> Self {
        self.mutate(move |node| {
            node.attach(CaretAnchorExt(anchor));
        })
    }

    fn ime_eligible<F>(self, f: F) -> Self
    where
        F: Fn() -> bool + 'static,
    {
        with_ime_handlers(self, move |handlers| handlers.set_eligible(Box::new(f)))
    }
}

fn with_ime_handlers(
    builder: NodeBuilder,
    mutate: impl FnOnce(&mut ImeHandlers) + 'static,
) -> NodeBuilder {
    builder.mutate(move |node| {
        if node.get_extension::<ImeHandlers>().is_none() {
            let _ = node.attach(ImeHandlers::default());
        }
        if let Some(handlers) = node.get_extension_mut::<ImeHandlers>() {
            mutate(handlers);
        }
    })
}

fn with_handlers(
    builder: NodeBuilder,
    mutate: impl FnOnce(&mut EventHandlers) + 'static,
) -> NodeBuilder {
    builder.mutate(move |node| mutate_handlers(node, mutate))
}

fn mutate_handlers(node: &mut Node, mutate: impl FnOnce(&mut EventHandlers)) {
    if node.get_extension::<EventHandlers>().is_none() {
        let _ = node.attach(EventHandlers::default());
    }
    if let Some(handlers) = node.get_extension_mut::<EventHandlers>() {
        mutate(handlers);
    }
}
