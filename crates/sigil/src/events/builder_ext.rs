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
use crate::events::handlers::KeyBinding;
use crate::events::ime_handlers::{CaretAnchorExt, ImeHandlers};
use crate::events::router::{GestureHandlers, PasteHandlers};
use crate::events::selection::SelectableText;

macro_rules! handler_methods {
    ($m:ident) => {
        $m! {
            pointer_down => EventHandlers.pointer_down: [Fn(&mut PointerEvent)],
            pointer_down_capture => EventHandlers.pointer_down_capture: [Fn(&mut PointerEvent)],
            pointer_up_capture => EventHandlers.pointer_up_capture: [Fn(&mut PointerEvent)],
            pointer_move => EventHandlers.pointer_move: [Fn(&mut PointerEvent)],
            pointer_move_capture => EventHandlers.pointer_move_capture: [Fn(&mut PointerEvent)],
            pointer_enter => EventHandlers.pointer_enter: [Fn(&mut PointerEvent)],
            pointer_leave => EventHandlers.pointer_leave: [Fn(&mut PointerEvent)],
            file_drop => EventHandlers.file_drop: [Fn(&FileDropEvent)],
            link => EventHandlers.link: [Fn(&str)],
            on_selection_copy => EventHandlers.selection_copy:
                [Fn(&str, &crate::AppResult<crate::clipboard::Delivery>)],
            on_scroll => EventHandlers.scroll: [Fn(&mut ScrollEvent)],
            key_any => EventHandlers.key_any: [Fn(&mut KeyEvent)],
            key_capture_any => EventHandlers.key_capture_any: [Fn(&mut KeyEvent)],
            focus_in => EventHandlers.focus_in: [Fn()],
            focus_out => EventHandlers.focus_out: [Fn()],
            on_ime => ImeHandlers.handlers: [Fn(&ImeEvent)],
            on_paste => PasteHandlers.handlers: [Fn(&mut PasteEvent)],
            on_gesture => GestureHandlers.handlers: [Fn(&mut GestureEvent)],
        }
        $m! {
            @keyed key => key, shortcut => shortcut, key_capture => key_capture,
        }
    };
}

macro_rules! declare {
    ($($name:ident => $ext:ident.$field:ident: [$($bound:tt)*],)*) => {
        $(#[must_use] fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static;)*
    };
    (@keyed $($name:ident => $field:ident,)*) => {
        $(#[must_use] fn $name<S, H>(self, shortcut: S, h: H) -> Self
        where S: Into<Shortcut>, H: Fn(&mut KeyEvent) + 'static;)*
    };
}

macro_rules! implement {
    ($($name:ident => $ext:ident.$field:ident: [$($bound:tt)*],)*) => {
        $(fn $name<H>(self, h: H) -> Self where H: $($bound)* + 'static {
            with_ext(self, move |ext: &mut $ext| ext.$field.push(Box::new(h)))
        })*
    };
    (@keyed $($name:ident => $field:ident,)*) => {
        $(fn $name<S, H>(self, shortcut: S, h: H) -> Self
        where S: Into<Shortcut>, H: Fn(&mut KeyEvent) + 'static {
            let shortcut = shortcut.into();
            with_handlers(self, move |handlers| {
                handlers.$field.push(KeyBinding { shortcut, handler: Box::new(h) });
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
            handlers.pointer_up.push(Box::new(h));
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
