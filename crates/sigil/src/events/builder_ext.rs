use std::cell::Cell;
use std::rc::Rc;

use crate::events::{
    FileDropEvent, GestureEvent, ImeEvent, KeyEvent, PointerEvent, ScrollEvent, Shortcut,
};
use crate::kernel::Action;
use crate::kernel::{Node, NodeBuilder};
use crate::layout::CaretGeometry;
use crate::reactive::Signal;

use crate::events::EventHandlers;
use crate::events::handlers::{EventKind, Handler, Phase};
use crate::events::ime_handlers::{CaretAnchorExt, ImeEligible};
use crate::events::selection::SelectableText;

macro_rules! handler_methods {
    ($m:ident) => {
        $m! {
            @table
            /// Runs `h` when a pointer button goes down on the node.
            pointer_down => PointerDown, Bubble, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when a pointer button goes down, before the node's children see it.
            pointer_down_capture => PointerDown, Capture, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when a pointer button goes up, before the node's children see it.
            pointer_up_capture => PointerUp, Capture, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when the pointer moves over the node.
            pointer_move => PointerMove, Bubble, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when the pointer moves, before the node's children see it.
            pointer_move_capture => PointerMove, Capture, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when the pointer enters the node.
            pointer_enter => PointerEnter, Bubble, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when the pointer leaves the node.
            pointer_leave => PointerLeave, Bubble, Pointer: [Fn(&mut PointerEvent)],
            /// Runs `h` when files are dragged over or dropped on the node.
            file_drop => FileDrop, Bubble, FileDrop: [Fn(&FileDropEvent)],
            /// Runs `h` with the target when a link inside the node is activated.
            link => Link, Bubble, Link: [Fn(&str)],
            /// Runs `h` with the copied text and the clipboard result after a selection copy.
            selection_copy => SelectionCopy, Bubble, SelectionCopy:
                [Fn(&str, &crate::AppResult<crate::clipboard::Delivery>)],
            /// Runs `h` when the node receives a scroll event.
            scroll => Scroll, Bubble, Scroll: [Fn(&mut ScrollEvent)],
            /// Runs `h` when focus enters the node or one of its children.
            focus_in => FocusIn, Bubble, Focus: [Fn()],
            /// Runs `h` when focus leaves the node or one of its children.
            focus_out => FocusOut, Bubble, Focus: [Fn()],
            /// Runs `h` for each input method event sent to the node.
            ime => Ime, Bubble, Ime: [Fn(&ImeEvent)],
            /// Runs `h` with pasted text; returning true stops the paste from reaching ancestors.
            paste => Paste, Bubble, Paste: [Fn(&str) -> bool],
            /// Runs `h` for each touchpad gesture over the node.
            gesture => Gesture, Bubble, Gesture: [Fn(&mut GestureEvent)],
        }
        $m! {
            @keyed
            /// Runs `h` when a key event matching `shortcut` reaches the node.
            key => Key, Bubble,
            /// Runs `h` when the key combination `shortcut` reaches the node.
            shortcut => Shortcut, Bubble,
            /// Runs `h` for a key event matching `shortcut` before the node's children see it.
            key_capture => Key, Capture,
        }
        $m! {
            @any
            /// Runs `h` for every key event that reaches the node.
            key_any => Bubble,
            /// Runs `h` for every key event before the node's children see it.
            key_capture_any => Capture,
        }
    };
}

pub(crate) use handler_methods;

macro_rules! implement {
    (@table $($(#[$doc:meta])* $name:ident => $kind:ident, $phase:ident, $variant:ident: [$($bound:tt)*],)*) => {
        $(pub(crate) fn $name<H>(node: NodeBuilder, h: H) -> NodeBuilder where H: $($bound)* + 'static {
            with_handlers(node, move |handlers| {
                handlers.push(EventKind::$kind, Phase::$phase, Handler::$variant(Box::new(h)));
            })
        })*
    };
    (@keyed $($(#[$doc:meta])* $name:ident => $kind:ident, $phase:ident,)*) => {
        $(pub(crate) fn $name<H>(node: NodeBuilder, shortcut: Shortcut, h: H) -> NodeBuilder
        where H: Fn(&mut KeyEvent) + 'static {
            with_handlers(node, move |handlers| {
                handlers.push(EventKind::$kind, Phase::$phase, Handler::Key(Some(shortcut), Box::new(h)));
            })
        })*
    };
    (@any $($(#[$doc:meta])* $name:ident => $phase:ident,)*) => {
        $(pub(crate) fn $name<H>(node: NodeBuilder, h: H) -> NodeBuilder where H: Fn(&mut KeyEvent) + 'static {
            with_handlers(node, move |handlers| {
                handlers.push(EventKind::Key, Phase::$phase, Handler::Key(None, Box::new(h)));
            })
        })*
    };
}

macro_rules! event_methods {
    (@table $($(#[$doc:meta])* $name:ident => $kind:ident, $phase:ident, $variant:ident: [$($bound:tt)*],)*) => {
        $($(#[$doc])* #[must_use] fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static {
            self.map_node(move |node| $crate::events::builder_ext::$name(node, h))
        })*
    };
    (@keyed $($(#[$doc:meta])* $name:ident => $kind:ident, $phase:ident,)*) => {
        $($(#[$doc])* #[must_use] fn $name<S, H>(self, shortcut: S, h: H) -> Self
        where S: Into<Shortcut>, H: Fn(&mut KeyEvent) + 'static {
            let shortcut = shortcut.into();
            self.map_node(move |node| $crate::events::builder_ext::$name(node, shortcut, h))
        })*
    };
    (@any $($(#[$doc:meta])* $name:ident => $phase:ident,)*) => {
        $($(#[$doc])* #[must_use] fn $name<H>(self, h: H) -> Self where H: Fn(&mut KeyEvent) + 'static {
            self.map_node(move |node| $crate::events::builder_ext::$name(node, h))
        })*
    };
}

pub(crate) use event_methods;

handler_methods!(implement);

pub(crate) fn pointer_up<H>(node: NodeBuilder, h: H) -> NodeBuilder
where
    H: Fn(&mut PointerEvent) + 'static,
{
    release(node.action(Action::Click), h, false)
}

pub(crate) fn track_release<H>(node: NodeBuilder, h: H) -> NodeBuilder
where
    H: Fn(&mut PointerEvent) + 'static,
{
    release(node, h, true)
}

fn release<H>(node: NodeBuilder, h: H, tracks: bool) -> NodeBuilder
where
    H: Fn(&mut PointerEvent) + 'static,
{
    with_handlers(node, move |handlers| {
        handlers.push_entry(
            EventKind::PointerUp,
            Phase::Bubble,
            Handler::Pointer(Box::new(h)),
            tracks,
        );
    })
}

pub(crate) fn ime_caret(node: NodeBuilder, anchor: Rc<Cell<Option<CaretGeometry>>>) -> NodeBuilder {
    node.mutate(move |node| {
        node.attach(CaretAnchorExt(anchor));
    })
}

pub(crate) fn ime_eligible<F>(node: NodeBuilder, f: F) -> NodeBuilder
where
    F: Fn() -> bool + 'static,
{
    node.mutate(move |node| {
        node.attach(ImeEligible(Box::new(f)));
    })
}

pub(crate) fn selectable_text(node: NodeBuilder, yes: bool) -> NodeBuilder {
    if yes { node.with(SelectableText) } else { node }
}

pub(crate) fn autofocus(node: NodeBuilder) -> NodeBuilder {
    node.with(crate::events::focus::Autofocus)
        .added(|_, id| crate::events::focus::request_focus(id))
}

pub(crate) fn focus_when(node: NodeBuilder, trigger: Signal<u64>) -> NodeBuilder {
    node.added(move |_, id| {
        crate::events::focus::focus_on_trigger(trigger, move || {
            crate::events::focus::request_focus(id);
        });
    })
}

fn with_ext<T: Default + 'static>(
    builder: NodeBuilder,
    mutate: impl FnOnce(&mut T) + 'static,
) -> NodeBuilder {
    builder.mutate(move |node| ext_mut(node, mutate))
}

fn ext_mut<T: Default + 'static>(node: &mut Node, mutate: impl FnOnce(&mut T)) {
    if node.get_extension::<T>().is_none() {
        let _ = node.attach(T::default());
    }
    if let Some(ext) = node.get_extension_mut::<T>() {
        mutate(ext);
    }
}

pub(crate) fn with_handlers(
    builder: NodeBuilder,
    mutate: impl FnOnce(&mut EventHandlers) + 'static,
) -> NodeBuilder {
    with_ext(builder, mutate)
}

pub(crate) fn mutate_handlers(node: &mut Node, mutate: impl FnOnce(&mut EventHandlers)) {
    ext_mut(node, mutate);
}
