use crate::clipboard::Delivery;
use crate::events::{FileDropEvent, KeyEvent, PointerEvent, PointerPhase, ScrollEvent, Shortcut};
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
    let handlers = node.get_extension::<EventHandlers>()?;
    let mut result = None;
    owner_by_id(node.owner, || result = Some(invoke(handlers)));
    result
}

pub(crate) type Callback<E> = Box<dyn Fn(&mut E) + 'static>;
type FocusCallback = Box<dyn Fn() + 'static>;
type ValueAdjustment = Box<dyn Fn(bool) + 'static>;
type A11yEditHandler = Box<dyn Fn(&A11yEdit) -> bool + 'static>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum A11yEdit {
    Number(f32),
    Text(String),
    Select { anchor: usize, focus: usize },
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
}

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
}

impl EventArg<'_> {
    const fn stopped(&self) -> bool {
        match self {
            Self::Pointer(event) => event.propagation_stopped(),
            Self::Key(event) => event.propagation_stopped(),
            Self::Scroll(event) => event.propagation_stopped(),
            Self::Focus | Self::FileDrop(_) | Self::Link(_) | Self::SelectionCopy(..) => false,
        }
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
            _ => return false,
        }
        true
    }
}

struct HandlerEntry {
    kind: EventKind,
    phase: Phase,
    handler: Handler,
}

#[derive(Default)]
pub(crate) struct EventHandlers {
    entries: Vec<HandlerEntry>,
    value_adjustment: Option<ValueAdjustment>,
    a11y_edit: Option<A11yEditHandler>,
}

impl EventHandlers {
    pub(crate) fn push(&mut self, kind: EventKind, phase: Phase, handler: Handler) {
        self.entries.push(HandlerEntry {
            kind,
            phase,
            handler,
        });
    }

    pub(crate) fn dispatch(&self, kind: EventKind, phase: Phase, arg: &mut EventArg<'_>) -> bool {
        let selected = |keyed: bool| {
            self.entries.iter().filter(move |entry| {
                entry.kind == kind
                    && entry.phase == phase
                    && entry.handler.shortcut().is_some() == keyed
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

    pub(crate) fn set_value_adjustment(&mut self, handler: ValueAdjustment) {
        self.value_adjustment = Some(handler);
    }

    pub(crate) fn set_a11y_edit(&mut self, handler: A11yEditHandler) {
        self.a11y_edit = Some(handler);
    }

    pub(crate) fn invoke_a11y_edit(&self, edit: &A11yEdit) -> bool {
        self.a11y_edit.as_ref().is_some_and(|handler| handler(edit))
    }

    pub(crate) fn invoke_value_adjustment(&self, increase: bool) -> bool {
        let Some(handler) = &self.value_adjustment else {
            return false;
        };
        handler(increase);
        true
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

pub(crate) fn until_stopped<E>(
    handlers: &[Callback<E>],
    event: &mut E,
    stopped: impl Fn(&E) -> bool,
) -> bool {
    for handler in handlers {
        handler(event);
        if stopped(event) {
            break;
        }
    }
    !handlers.is_empty()
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
    let gated = !kind.tracks_state();
    dispatch_gated(cx, id, gated, |handlers| {
        handlers.dispatch(kind, phase, &mut arg)
    })
}
