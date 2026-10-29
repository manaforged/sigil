use std::cell::Cell;
use std::rc::Rc;

use crate::events::{
    FileDropEvent, GestureEvent, ImeEvent, KeyEvent, PasteEvent, PointerEvent, ScrollEvent,
    Shortcut,
};
use crate::kernel::{Node, NodeBuilder};
use crate::layout::CaretGeometry;
use crate::reactive::Signal;
use accesskit::Action;

use crate::events::EventHandlers;
use crate::events::handlers::{EventKind, Handler, Phase};
use crate::events::ime_handlers::{CaretAnchorExt, ImeHandlers};
use crate::events::router::{GestureHandlers, PasteHandlers};
use crate::events::selection::SelectableText;

macro_rules! handler_methods {
    ($m:ident) => {
        $m! {
            @table
            pointer_down => PointerDown, Bubble, Pointer: [Fn(&mut PointerEvent)],
            pointer_down_capture => PointerDown, Capture, Pointer: [Fn(&mut PointerEvent)],
            pointer_up_capture => PointerUp, Capture, Pointer: [Fn(&mut PointerEvent)],
            pointer_move => PointerMove, Bubble, Pointer: [Fn(&mut PointerEvent)],
            pointer_move_capture => PointerMove, Capture, Pointer: [Fn(&mut PointerEvent)],
            pointer_enter => PointerEnter, Bubble, Pointer: [Fn(&mut PointerEvent)],
            pointer_leave => PointerLeave, Bubble, Pointer: [Fn(&mut PointerEvent)],
            file_drop => FileDrop, Bubble, FileDrop: [Fn(&FileDropEvent)],
            link => Link, Bubble, Link: [Fn(&str)],
            on_selection_copy => SelectionCopy, Bubble, SelectionCopy:
                [Fn(&str, &crate::AppResult<crate::clipboard::Delivery>)],
            on_scroll => Scroll, Bubble, Scroll: [Fn(&mut ScrollEvent)],
            focus_in => FocusIn, Bubble, Focus: [Fn()],
            focus_out => FocusOut, Bubble, Focus: [Fn()],
        }
        $m! {
            @ext
            on_ime => ImeHandlers.handlers: [Fn(&ImeEvent)],
            on_paste => PasteHandlers.handlers: [Fn(&mut PasteEvent)],
            on_gesture => GestureHandlers.handlers: [Fn(&mut GestureEvent)],
        }
        $m! {
            @keyed
            key => Key, Bubble,
            shortcut => Shortcut, Bubble,
            key_capture => Key, Capture,
        }
        $m! {
            @any
            key_any => Bubble,
            key_capture_any => Capture,
        }
    };
}

macro_rules! declare {
    (@table $($name:ident => $kind:ident, $phase:ident, $variant:ident: [$($bound:tt)*],)*) => {
        $(#[must_use] fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static;)*
    };
    (@ext $($name:ident => $ext:ident.$field:ident: [$($bound:tt)*],)*) => {
        $(#[must_use] fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static;)*
    };
    (@keyed $($name:ident => $kind:ident, $phase:ident,)*) => {
        $(#[must_use] fn $name<S, H>(self, shortcut: S, h: H) -> Self
        where S: Into<Shortcut>, H: Fn(&mut KeyEvent) + 'static;)*
    };
    (@any $($name:ident => $phase:ident,)*) => {
        $(#[must_use] fn $name<H>(self, h: H) -> Self where H: Fn(&mut KeyEvent) + 'static;)*
    };
}

macro_rules! implement {
    (@table $($name:ident => $kind:ident, $phase:ident, $variant:ident: [$($bound:tt)*],)*) => {
        $(fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static {
            with_handlers(self, move |handlers| {
                handlers.push(EventKind::$kind, Phase::$phase, Handler::$variant(Box::new(h)));
            })
        })*
    };
    (@ext $($name:ident => $ext:ident.$field:ident: [$($bound:tt)*],)*) => {
        $(fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static {
            with_ext(self, move |ext: &mut $ext| ext.$field.push(Box::new(h)))
        })*
    };
    (@keyed $($name:ident => $kind:ident, $phase:ident,)*) => {
        $(fn $name<S, H>(self, shortcut: S, h: H) -> Self
        where S: Into<Shortcut>, H: Fn(&mut KeyEvent) + 'static {
            let shortcut = Some(shortcut.into());
            with_handlers(self, move |handlers| {
                handlers.push(EventKind::$kind, Phase::$phase, Handler::Key(shortcut, Box::new(h)));
            })
        })*
    };
    (@any $($name:ident => $phase:ident,)*) => {
        $(fn $name<H>(self, h: H) -> Self where H: Fn(&mut KeyEvent) + 'static {
            with_handlers(self, move |handlers| {
                handlers.push(EventKind::Key, Phase::$phase, Handler::Key(None, Box::new(h)));
            })
        })*
    };
}

pub trait NodeEventsExt {
    handler_methods!(declare);

    #[must_use]
    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static;

    #[must_use]
    fn ime_caret(self, anchor: Rc<Cell<Option<CaretGeometry>>>) -> Self;

    #[must_use]
    fn ime_eligible<F>(self, f: F) -> Self
    where
        F: Fn() -> bool + 'static;

    #[must_use]
    fn selectable_text(self, yes: bool) -> Self;

    #[must_use]
    fn autofocus(self) -> Self;

    #[must_use]
    fn focus_when(self, trigger: Signal<u64>) -> Self;
}

impl NodeEventsExt for NodeBuilder {
    handler_methods!(implement);

    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        with_handlers(self.action(Action::Click), move |handlers| {
            handlers.push(
                EventKind::PointerUp,
                Phase::Bubble,
                Handler::Pointer(Box::new(h)),
            );
        })
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
        with_ext(self, move |ime: &mut ImeHandlers| {
            ime.eligible = Some(Box::new(f));
        })
    }

    fn selectable_text(self, yes: bool) -> Self {
        if yes { self.with(SelectableText) } else { self }
    }

    fn autofocus(self) -> Self {
        self.with(crate::events::focus::Autofocus)
            .on_added(|_, id| crate::events::focus::request_focus(id))
    }

    fn focus_when(self, trigger: Signal<u64>) -> Self {
        self.on_added(move |_, id| {
            crate::events::focus::focus_on_trigger(trigger, move || {
                crate::events::focus::request_focus(id);
            });
        })
    }
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
