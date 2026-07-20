//! Builder extension trait for installing event handlers.

use std::cell::Cell;
use std::rc::Rc;

use sigil_hal::{ImeEvent, KeyEvent, PointerEvent, ScrollEvent, Shortcut};
use sigil_kernel::{Node, NodeBuilder};
use sigil_layout::CaretGeometry;

use crate::EventHandlers;
use crate::ime_handlers::{CaretAnchorExt, ImeHandlers};

/// Adds event-handler methods to [`NodeBuilder`].
pub trait NodeEventsExt {
    /// Run `h` during the pointer-down bubble/target phase.
    #[must_use]
    fn pointer_down<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-down capture phase.
    #[must_use]
    fn pointer_down_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-up bubble/target phase.
    #[must_use]
    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-up capture phase.
    #[must_use]
    fn pointer_up_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-move bubble/target phase.
    #[must_use]
    fn pointer_move<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-move capture phase.
    #[must_use]
    fn pointer_move_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` when the pointer enters this node's hit path.
    #[must_use]
    fn pointer_enter<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` when the pointer leaves this node's hit path.
    #[must_use]
    fn pointer_leave<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` when a scroll event lands on this node or an ancestor.
    /// Bubbles target-first; handlers can call `event.stop_propagation()`
    /// to prevent the event from reaching outer scroll containers.
    #[must_use]
    fn on_scroll<H>(self, h: H) -> Self
    where
        H: Fn(&mut ScrollEvent) + 'static;

    /// Run `h` when a matching key event reaches this node.
    #[must_use]
    fn key<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static;

    /// Run `h` when any key event reaches this node.
    #[must_use]
    fn key_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static;

    /// Run `h` during key-event capture for a matching shortcut.
    #[must_use]
    fn key_capture<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static;

    /// Run `h` when this node receives focus.
    #[must_use]
    fn focus_in<H>(self, h: H) -> Self
    where
        H: Fn() + 'static;

    /// Run `h` when this node loses focus.
    #[must_use]
    fn focus_out<H>(self, h: H) -> Self
    where
        H: Fn() + 'static;

    /// Run `h` when an IME composition event reaches this node while it is
    /// focused. See `EventRouter::route_ime`.
    #[must_use]
    fn on_ime<H>(self, h: H) -> Self
    where
        H: Fn(&ImeEvent) + 'static;

    /// Publish the node-relative caret anchor used to resolve
    /// `EventRouter::ime_caret_rect`. The widget updates the shared cell
    /// from its own commit/preedit logic; the router combines it with the
    /// node's laid-out origin at end-of-Layout.
    #[must_use]
    fn ime_caret(self, anchor: Rc<Cell<Option<CaretGeometry>>>) -> Self;

    /// Register a live "am I currently eligible for IME composition"
    /// query, consulted by the router before every dispatch and by
    /// `EventRouter::focus_wants_ime`. Lets a widget's own
    /// disabled/read-only state gate IME independently of whether
    /// `.on_ime(...)` was ever registered. Defaults to always-eligible
    /// when never called.
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
        with_handlers(self, move |handlers| handlers.add_pointer_up(Box::new(h)))
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
