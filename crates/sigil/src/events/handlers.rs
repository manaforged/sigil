use crate::clipboard::Delivery;
use crate::events::{
    FileDropEvent, GestureEvent, ImeEvent, KeyEvent, PointerEvent, PointerPhase, ScrollEvent,
    Shortcut,
};
use crate::kernel::{Cx, NodeId};
use crate::reactive::owner_by_id;

pub(crate) fn dispatch<T>(
    cx: &Cx,
    id: NodeId,
    invoke: impl FnOnce(&EventHandlers) -> T,
) -> Option<T> {
    dispatch_gated(cx, id, true, invoke)
}

fn dispatch_gated<T>(
    cx: &Cx,
    id: NodeId,
    gated: bool,
    invoke: impl FnOnce(&EventHandlers) -> T,
) -> Option<T> {
    let node = cx.get(id)?;
    if gated && node.a11y.is_disabled() {
        return None;
    }
    let handlers = node.extension::<EventHandlers>()?;
    let mut result = None;
    owner_by_id(node.owner, || result = Some(invoke(handlers)));
    result
}

pub(crate) type Callback<E> = Box<dyn Fn(&mut E) + 'static>;
type FocusCallback = Box<dyn Fn() + 'static>;
type A11yEditHandler = Box<dyn Fn(&A11yEdit) -> bool + 'static>;

/// Stage of a pointer drag gesture delivered to [`Widget::pointer_drag`](crate::widgets::Widget::pointer_drag).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DragPhase {
    /// Primary pointer press started the drag.
    Start,
    /// Pointer moved while captured by the drag.
    Move,
    /// Primary pointer release ended the drag.
    End,
    /// Pointer cancel or lost capture aborted the drag.
    Cancel,
}

/// A key's change to a ranged value, delivered to [`Widget::step_keys`](crate::widgets::Widget::step_keys).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ValueStep {
    /// Move by this many small steps; a negative count moves down.
    Unit(f32),
    /// Move by this many pages; a negative count moves down.
    Page(f32),
    /// Jump to the lowest value.
    Start,
    /// Jump to the highest value.
    End,
}

/// An accessibility edit request dispatched to a widget handler.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum A11yEdit {
    /// Step the numeric value upward.
    Increment,
    /// Step the numeric value downward.
    Decrement,
    /// Set the value to a specific number.
    Number(f32),
    /// Replace the text value.
    Text(String),
    /// Update the text selection range in UTF-8 byte offsets.
    Select {
        /// Byte offset of the stationary end of the selection.
        anchor: usize,
        /// Byte offset of the moving end of the selection.
        focus: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventKind {
    PointerDown,
    PointerUp,
    PointerMove,
    PointerEnter,
    PointerLeave,
    Scroll,
    Key,
    Shortcut,
    FocusIn,
    FocusOut,
    FileDrop,
    Link,
    SelectionCopy,
    Paste,
    Gesture,
    Ime,
}

impl EventKind {
    const fn tracks_state(self) -> bool {
        matches!(
            self,
            Self::PointerEnter | Self::PointerLeave | Self::FocusOut
        )
    }

    pub(crate) const fn pointer(phase: PointerPhase) -> Self {
        match phase {
            PointerPhase::Down => Self::PointerDown,
            PointerPhase::Up | PointerPhase::Cancel => Self::PointerUp,
            PointerPhase::Move => Self::PointerMove,
            PointerPhase::Enter => Self::PointerEnter,
            PointerPhase::Leave => Self::PointerLeave,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Capture,
    Bubble,
}

impl Phase {
    pub(crate) const fn of(capture: bool) -> Self {
        if capture { Self::Capture } else { Self::Bubble }
    }
}

pub(crate) enum Handler {
    Pointer(Callback<PointerEvent>),
    Key(Option<Shortcut>, Callback<KeyEvent>),
    Scroll(Callback<ScrollEvent>),
    Focus(FocusCallback),
    FileDrop(FileDropCallback),
    Link(LinkCallback),
    SelectionCopy(CopyCallback),
    Paste(PasteCallback),
    Gesture(Callback<GestureEvent>),
    Ime(ImeCallback),
}

type PasteCallback = Box<dyn Fn(&str) -> bool + 'static>;
type ImeCallback = Box<dyn Fn(&ImeEvent) + 'static>;
type FileDropCallback = Box<dyn Fn(&FileDropEvent) + 'static>;
type LinkCallback = Box<dyn Fn(&str) + 'static>;
type CopyCallback = Box<dyn Fn(&str, &crate::AppResult<Delivery>) + 'static>;

pub(crate) enum EventArg<'a> {
    Pointer(&'a mut PointerEvent),
    Key(&'a mut KeyEvent),
    Scroll(&'a mut ScrollEvent),
    Focus,
    FileDrop(&'a FileDropEvent),
    Link(&'a str),
    SelectionCopy(&'a str, &'a crate::AppResult<Delivery>),
    Paste(&'a str, &'a mut bool),
    Gesture(&'a mut GestureEvent),
    Ime(&'a ImeEvent),
}

impl EventArg<'_> {
    const fn stopped(&self) -> bool {
        match self {
            Self::Pointer(event) => event.propagation_stopped(),
            Self::Key(event) => event.propagation_stopped(),
            Self::Scroll(event) => event.propagation_stopped(),
            Self::Gesture(event) => event.propagation_stopped(),
            Self::Paste(_, handled) => **handled,
            Self::Focus
            | Self::FileDrop(_)
            | Self::Link(_)
            | Self::SelectionCopy(..)
            | Self::Ime(_) => false,
        }
    }

    const fn ends_session(&self) -> bool {
        matches!(self, Self::Ime(ImeEvent::Disabled))
    }
}

impl Handler {
    const fn shortcut(&self) -> Option<&Shortcut> {
        match self {
            Self::Key(shortcut, _) => shortcut.as_ref(),
            _ => None,
        }
    }

    fn call(&self, arg: &mut EventArg<'_>) -> bool {
        match (self, arg) {
            (Self::Pointer(h), EventArg::Pointer(event)) => h(event),
            (Self::Key(shortcut, h), EventArg::Key(event)) => {
                if shortcut.as_ref().is_some_and(|s| !s.matches(event)) {
                    return false;
                }
                h(event);
            }
            (Self::Scroll(h), EventArg::Scroll(event)) => h(event),
            (Self::Focus(h), EventArg::Focus) => h(),
            (Self::FileDrop(h), EventArg::FileDrop(event)) => h(event),
            (Self::Link(h), EventArg::Link(href)) => h(href),
            (Self::SelectionCopy(h), EventArg::SelectionCopy(text, result)) => h(text, result),
            (Self::Paste(h), EventArg::Paste(text, handled)) => **handled |= h(text),
            (Self::Gesture(h), EventArg::Gesture(event)) => h(event),
            (Self::Ime(h), EventArg::Ime(event)) => h(event),
            _ => return false,
        }
        true
    }
}

struct HandlerEntry {
    kind: EventKind,
    phase: Phase,
    handler: Handler,
    tracks: bool,
}

#[derive(Default)]
pub(crate) struct EventHandlers {
    entries: Vec<HandlerEntry>,
    a11y_edit: Option<A11yEditHandler>,
}

impl EventHandlers {
    pub(crate) fn push(&mut self, kind: EventKind, phase: Phase, handler: Handler) {
        self.push_entry(kind, phase, handler, false);
    }

    pub(crate) fn push_entry(
        &mut self,
        kind: EventKind,
        phase: Phase,
        handler: Handler,
        tracks: bool,
    ) {
        self.entries.push(HandlerEntry {
            kind,
            phase,
            handler,
            tracks,
        });
    }

    pub(crate) fn dispatch(&self, kind: EventKind, phase: Phase, arg: &mut EventArg<'_>) -> bool {
        self.dispatch_to(kind, phase, arg, false)
    }

    fn dispatch_to(
        &self,
        kind: EventKind,
        phase: Phase,
        arg: &mut EventArg<'_>,
        blocked: bool,
    ) -> bool {
        let selected = |keyed: bool| {
            self.entries.iter().filter(move |entry| {
                entry.kind == kind
                    && entry.phase == phase
                    && entry.handler.shortcut().is_some() == keyed
                    && (entry.tracks || !blocked)
            })
        };
        let mut invoked = false;
        for entry in selected(true).chain(selected(false)) {
            invoked |= entry.handler.call(arg);
            if arg.stopped() {
                break;
            }
        }
        invoked
    }

    pub(crate) fn handles(&self, kind: EventKind) -> bool {
        self.entries.iter().any(|entry| entry.kind == kind)
    }

    pub(crate) fn owns_pointer_drag(&self) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                (entry.kind, entry.phase),
                (EventKind::PointerDown, Phase::Capture) | (EventKind::PointerMove, _)
            )
        })
    }

    pub(crate) fn shortcuts(&self) -> impl Iterator<Item = &Shortcut> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == EventKind::Shortcut)
            .filter_map(|entry| entry.handler.shortcut())
    }

    pub(crate) fn set_a11y_edit(&mut self, handler: A11yEditHandler) {
        self.a11y_edit = Some(handler);
    }

    pub(crate) fn invoke_a11y_edit(&self, edit: &A11yEdit) -> bool {
        self.a11y_edit.as_ref().is_some_and(|handler| handler(edit))
    }

    pub(crate) fn add_key(&mut self, shortcut: Shortcut, handler: Callback<KeyEvent>) {
        self.push(
            EventKind::Key,
            Phase::Bubble,
            Handler::Key(Some(shortcut), handler),
        );
    }

    pub(crate) fn add_focus_in(&mut self, handler: FocusCallback) {
        self.push(EventKind::FocusIn, Phase::Bubble, Handler::Focus(handler));
    }

    pub(crate) fn add_focus_out(&mut self, handler: FocusCallback) {
        self.push(EventKind::FocusOut, Phase::Bubble, Handler::Focus(handler));
    }
}

impl core::fmt::Debug for EventHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EventHandlers").finish_non_exhaustive()
    }
}

pub(crate) fn fire(
    cx: &Cx,
    id: NodeId,
    kind: EventKind,
    phase: Phase,
    mut arg: EventArg<'_>,
) -> Option<bool> {
    let blocked = !kind.tracks_state() && !arg.ends_session() && cx.get(id)?.a11y.is_disabled();
    dispatch_gated(cx, id, false, |handlers| {
        handlers.dispatch_to(kind, phase, &mut arg, blocked)
    })
}
