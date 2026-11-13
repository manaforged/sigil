use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::events::{
    A11yEdit, DragPhase, FileDropEvent, GestureEvent, ImeEvent, KeyEvent, PointerEvent,
    ScrollEvent, Shortcut,
};
use crate::kernel::Action;
use crate::kernel::{Node, NodeBuilder};
use crate::layout::CaretGeometry;
use crate::reactive::Signal;
use crate::widgets::interaction::{is_cancel, is_primary};

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

pub(crate) fn shortcut<H>(node: NodeBuilder, shortcut: Shortcut, h: H) -> NodeBuilder
where
    H: Fn(&mut KeyEvent) + 'static,
{
    let a11y_shortcut = shortcut.clone();
    with_handlers(
        node.mutate(move |node| node.a11y.keyboard_shortcut = Some(a11y_shortcut)),
        move |handlers| {
            handlers.push(
                EventKind::Shortcut,
                Phase::Bubble,
                Handler::Key(Some(shortcut), Box::new(h)),
            );
        },
    )
}

pub(crate) fn accessible_shortcut(node: NodeBuilder, shortcut: Shortcut) -> NodeBuilder {
    node.mutate(move |node| node.a11y.keyboard_shortcut = Some(shortcut))
}

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
        node.attach(CaretAnchorExt(Box::new(move || anchor.get())));
    })
}

pub(crate) fn ime_caret_signal(
    node: NodeBuilder,
    caret: Signal<Option<CaretGeometry>>,
) -> NodeBuilder {
    node.mutate(move |node| {
        node.attach(CaretAnchorExt(Box::new(move || caret.get())));
    })
}

pub(crate) fn track_hovered(node: NodeBuilder, out: Signal<bool>) -> NodeBuilder {
    let node = pointer_enter(node, move |_| {
        let _ = out.try_set(true);
    });
    pointer_leave(node, move |_| {
        let _ = out.try_set(false);
    })
}

pub(crate) fn track_focused(node: NodeBuilder, out: Signal<bool>) -> NodeBuilder {
    let node = focus_in(node, move || {
        let _ = out.try_set(true);
    });
    focus_out(node, move || {
        let _ = out.try_set(false);
    })
}

pub(crate) fn track_pressed(node: NodeBuilder, out: Signal<bool>) -> NodeBuilder {
    let node = pointer_down(node, move |event| {
        if is_primary(event) {
            let _ = out.try_set(true);
        }
    });
    let node = pointer_leave(node, move |_| {
        let _ = out.try_set(false);
    });
    track_release(node, move |event| {
        if is_primary(event) || is_cancel(event) {
            let _ = out.try_set(false);
        }
    })
}

pub(crate) fn pointer_drag(
    node: NodeBuilder,
    handler: impl FnMut(DragPhase, &mut PointerEvent) + 'static,
) -> NodeBuilder {
    let active = Rc::new(Cell::new(false));
    let handler = Rc::new(RefCell::new(handler));
    let (down_active, move_active) = (Rc::clone(&active), Rc::clone(&active));
    let (down_handler, move_handler) = (Rc::clone(&handler), Rc::clone(&handler));
    let node = pointer_down(node, move |event| {
        if !is_primary(event) {
            return;
        }
        (*down_handler.borrow_mut())(DragPhase::Start, event);
        if !event.default_prevented() {
            down_active.set(true);
            event.capture_pointer();
        }
    });
    let node = pointer_move(node, move |event| {
        if move_active.get() && event.is_captured() {
            (*move_handler.borrow_mut())(DragPhase::Move, event);
        }
    });
    track_release(node, move |event| {
        let cancel = is_cancel(event);
        if (is_primary(event) || cancel) && active.get() {
            active.set(false);
            let phase = if cancel {
                DragPhase::Cancel
            } else {
                DragPhase::End
            };
            (*handler.borrow_mut())(phase, event);
        }
    })
}

pub(crate) fn a11y_edit(
    node: NodeBuilder,
    handler: impl Fn(&A11yEdit) -> bool + 'static,
) -> NodeBuilder {
    with_handlers(node, move |handlers| {
        handlers.set_a11y_edit(Box::new(handler));
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
        .added(|_, id| id.focus())
}

pub(crate) fn focus_when(node: NodeBuilder, trigger: Signal<u64>) -> NodeBuilder {
    node.added(move |_, id| {
        crate::events::focus::focus_on_trigger(trigger, move || {
            id.focus();
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
    if node.extension::<T>().is_none() {
        let _ = node.attach(T::default());
    }
    if let Some(ext) = node.extension_mut::<T>() {
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
