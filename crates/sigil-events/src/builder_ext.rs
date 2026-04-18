//! Builder extension trait for installing event handlers.

use sigil_core::{Node, NodeBuilder};
use sigil_hal::{KeyEvent, PointerEvent, Shortcut};

use crate::EventHandlers;

/// Adds event-handler methods to [`NodeBuilder`].
pub trait NodeEventsExt {
    /// Run `h` during the pointer-down bubble/target phase.
    #[must_use]
    fn on_pointer_down<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-down capture phase.
    #[must_use]
    fn on_pointer_down_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-up bubble/target phase.
    #[must_use]
    fn on_pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-up capture phase.
    #[must_use]
    fn on_pointer_up_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-move bubble/target phase.
    #[must_use]
    fn on_pointer_move<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` during the pointer-move capture phase.
    #[must_use]
    fn on_pointer_move_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` when the pointer enters this node's hit path.
    #[must_use]
    fn on_pointer_enter<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` when the pointer leaves this node's hit path.
    #[must_use]
    fn on_pointer_leave<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    /// Run `h` when a matching key event reaches this node.
    #[must_use]
    fn on_key<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static;

    /// Run `h` when any key event reaches this node.
    #[must_use]
    fn on_key_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static;

    /// Run `h` during key-event capture for a matching shortcut.
    #[must_use]
    fn on_key_capture<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static;

    /// Run `h` when this node receives focus.
    #[must_use]
    fn on_focus_in<H>(self, h: H) -> Self
    where
        H: Fn() + 'static;

    /// Run `h` when this node loses focus.
    #[must_use]
    fn on_focus_out<H>(self, h: H) -> Self
    where
        H: Fn() + 'static;
}

impl NodeEventsExt for NodeBuilder {
    fn on_pointer_down<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_pointer_down(Box::new(h)))
    }

    fn on_pointer_down_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_down_capture(Box::new(h));
        })
    }

    fn on_pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_pointer_up(Box::new(h)))
    }

    fn on_pointer_up_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_up_capture(Box::new(h));
        })
    }

    fn on_pointer_move<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_pointer_move(Box::new(h)))
    }

    fn on_pointer_move_capture<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_move_capture(Box::new(h));
        })
    }

    fn on_pointer_enter<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_enter(Box::new(h));
        })
    }

    fn on_pointer_leave<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self, move |handlers| {
            handlers.add_pointer_leave(Box::new(h));
        })
    }

    fn on_key<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        with_handlers(self, move |handlers| {
            handlers.add_key(shortcut, Box::new(h));
        })
    }

    fn on_key_any<H>(self, h: H) -> Self
    where
        H: Fn(&mut KeyEvent) + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_key_any(Box::new(h)))
    }

    fn on_key_capture<S, H>(self, shortcut: S, h: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(&mut KeyEvent) + 'static,
    {
        let shortcut = shortcut.into();
        with_handlers(self, move |handlers| {
            handlers.add_key_capture(shortcut, Box::new(h));
        })
    }

    fn on_focus_in<H>(self, h: H) -> Self
    where
        H: Fn() + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_focus_in(Box::new(h)))
    }

    fn on_focus_out<H>(self, h: H) -> Self
    where
        H: Fn() + 'static,
    {
        with_handlers(self, move |handlers| handlers.add_focus_out(Box::new(h)))
    }
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
