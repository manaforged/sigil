use std::cell::RefCell;
use std::rc::Rc;

use crate::events::{MouseButtons, PointerEvent};
use crate::kernel::{NodeBuilder, container};
use crate::reactive::{Signal, derived, signal};
use crate::style::{BORDER_WIDTH, Radius, StyleAtom, Token};
use crate::widgets::Widget;
use crate::widgets::interaction::{is_cancel, is_primary};

const DRAG_THRESHOLD_PX: f32 = 4.0;

const DROP_TARGET_CLASS: &str = "bg-accent-4";

const GHOST_OFFSET: (f32, f32) = (12.0, 10.0);

type Armed<T> = Rc<RefCell<Option<((f32, f32), T)>>>;
type Accepts<T> = Rc<dyn Fn(&T) -> bool>;
type OnDrop<T> = Rc<dyn Fn(T)>;

/// Shared state of one drag interaction.
pub struct DragState<T: 'static> {
    position: Signal<(f32, f32)>,
    payload: Signal<Option<T>>,
    targets: Rc<RefCell<DropRegistry<T>>>,
    armed: Armed<T>,
}

impl<T: 'static> core::fmt::Debug for DragState<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DragState")
            .field("position", &self.position)
            .field("payload", &self.payload)
            .finish_non_exhaustive()
    }
}

struct DropRegistry<T: 'static> {
    targets: Vec<DropTarget<T>>,
    next_id: usize,
}

struct DropTarget<T: 'static> {
    id: usize,
    bounds: Signal<(f32, f32, f32, f32)>,
    highlight: Signal<bool>,
    accepts: Accepts<T>,
    on_drop: OnDrop<T>,
}

struct TargetGuard<T: 'static> {
    registry: Rc<RefCell<DropRegistry<T>>>,
    id: usize,
}

impl<T: 'static> Drop for TargetGuard<T> {
    fn drop(&mut self) {
        if let Ok(mut registry) = self.registry.try_borrow_mut() {
            registry.targets.retain(|target| target.id != self.id);
        }
    }
}

impl<T: 'static> Clone for DragState<T> {
    fn clone(&self) -> Self {
        Self {
            position: self.position,
            payload: self.payload,
            targets: Rc::clone(&self.targets),
            armed: Rc::clone(&self.armed),
        }
    }
}

impl<T: Clone + 'static> DragState<T> {
    /// Creates a drag state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: signal((0.0, 0.0)),
            payload: signal(None),
            targets: Rc::new(RefCell::new(DropRegistry {
                targets: Vec::new(),
                next_id: 0,
            })),
            armed: Rc::new(RefCell::new(None)),
        }
    }

    /// Returns whether this drag state is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.payload.try_peek().flatten().is_some()
    }

    /// Returns the position.
    #[must_use]
    pub fn position(&self) -> (f32, f32) {
        self.position.try_peek().unwrap_or_default()
    }

    /// Returns the payload.
    #[must_use]
    pub fn payload(&self) -> Option<T> {
        self.payload.try_peek().flatten()
    }

    /// Cancels the current drag.
    pub fn cancel(&self) {
        self.end();
    }

    fn begin(&self, payload: T, at: (f32, f32)) {
        self.payload.set_always(Some(payload));
        self.position.set(at);
        self.update_hover(at);
    }

    fn settle(&self, at: (f32, f32)) {
        let drop = self.payload().and_then(|payload| {
            self.targets
                .borrow()
                .targets
                .iter()
                .rev()
                .find(|target| contains(target.bounds.peek(), at) && (target.accepts)(&payload))
                .map(|target| (Rc::clone(&target.on_drop), payload))
        });
        self.end();
        if let Some((on_drop, payload)) = drop {
            on_drop(payload);
        }
    }

    fn end(&self) {
        self.payload.set_always(None);
        self.update_hover((0.0, 0.0));
    }

    fn update_hover(&self, at: (f32, f32)) {
        let payload = self.payload();
        let registry = self.targets.borrow();
        let mut claimed = false;
        for target in registry.targets.iter().rev() {
            let hover = !claimed
                && payload
                    .as_ref()
                    .is_some_and(|p| contains(target.bounds.peek(), at) && (target.accepts)(p));
            claimed |= hover;
            set_flag(target.highlight, hover);
        }
    }

    fn register(
        &self,
        bounds: Signal<(f32, f32, f32, f32)>,
        highlight: Signal<bool>,
        accepts: Accepts<T>,
        on_drop: OnDrop<T>,
    ) -> TargetGuard<T> {
        let mut registry = self.targets.borrow_mut();
        let id = registry.next_id;
        registry.next_id += 1;
        registry.targets.push(DropTarget {
            id,
            bounds,
            highlight,
            accepts,
            on_drop,
        });
        TargetGuard {
            registry: Rc::clone(&self.targets),
            id,
        }
    }
}

impl<T: Clone + 'static> Default for DragState<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn source<T: Clone + 'static>(
    node: NodeBuilder,
    state: DragState<T>,
    payload: T,
) -> NodeBuilder {
    node.pointer_down(arm(state.clone(), payload))
        .pointer_move(drive(state.clone()))
        .pointer_up(release(state))
}

pub(crate) fn target<T: Clone + 'static>(
    node: NodeBuilder,
    state: DragState<T>,
    accepts: impl Fn(&T) -> bool + 'static,
    on_drop: impl Fn(T) + 'static,
) -> NodeBuilder {
    let bounds = signal((0.0, 0.0, 0.0, 0.0));
    let highlight = signal(false);
    let guard = state.register(bounds, highlight, Rc::new(accepts), Rc::new(on_drop));
    node.pointer_move(drive(state.clone()))
        .pointer_up(release(state))
        .bounds_reactive(bounds)
        .class_when(highlight, DROP_TARGET_CLASS)
        .mutate(move |node| {
            let _ = node.attach(guard);
        })
}

/// The node that follows the pointer.
#[must_use]
pub fn drag_ghost<T, N>(state: DragState<T>, render: impl Fn(&T) -> N + 'static) -> NodeBuilder
where
    T: Clone + 'static,
    N: crate::kernel::IntoNode + 'static,
{
    let pos_state = state.clone();
    let pos = derived(move || {
        let (x, y) = pos_state.position.get();
        (x + GHOST_OFFSET.0, y + GHOST_OFFSET.1)
    });
    container()
        .hittable(false)
        .position_reactive(pos)
        .children_reactive(move || ghost_content(&state, &render))
}

fn ghost_content<T, N>(state: &DragState<T>, render: &dyn Fn(&T) -> N) -> Vec<NodeBuilder>
where
    T: Clone + 'static,
    N: crate::kernel::IntoNode + 'static,
{
    let Some(payload) = state.payload.get() else {
        return Vec::new();
    };
    vec![
        container()
            .class("px-8 py-4")
            .class([
                Radius::Md.atom(),
                Token::Accent3.background(),
                Token::Accent12.text(),
                StyleAtom::BorderWidth(BORDER_WIDTH),
                Token::Accent6.border(),
            ])
            .child(render(&payload)),
    ]
}

fn arm<T: Clone + 'static>(
    state: DragState<T>,
    payload: T,
) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        if is_primary(event) {
            let at = (event.window_position.x, event.window_position.y);
            state.armed.replace(Some((at, payload.clone())));
            event.capture_pointer();
        }
    }
}

fn drive<T: Clone + 'static>(state: DragState<T>) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        let at = (event.window_position.x, event.window_position.y);
        if state.is_active() {
            if !event.is_captured() {
                if !event.buttons.contains(MouseButtons::LEFT) {
                    state.cancel();
                    return;
                }
                event.capture_pointer();
            }
            state.position.set(at);
            state.update_hover(at);
            event.stop_propagation();
            return;
        }
        if !event.is_captured() {
            return;
        }
        let ready = state
            .armed
            .borrow()
            .as_ref()
            .filter(|(down, _)| (at.0 - down.0).hypot(at.1 - down.1) >= DRAG_THRESHOLD_PX)
            .map(|(_, payload)| payload.clone());
        if let Some(payload) = ready {
            state.armed.replace(None);
            state.begin(payload, at);
            event.stop_propagation();
        }
    }
}

fn release<T: Clone + 'static>(state: DragState<T>) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        let cancelled = is_cancel(event);
        if !cancelled && !is_primary(event) {
            return;
        }
        state.armed.replace(None);
        if !state.is_active() {
            return;
        }
        if cancelled {
            state.end();
        } else {
            state.settle((event.window_position.x, event.window_position.y));
        }
        event.stop_propagation();
    }
}

fn contains(bounds: (f32, f32, f32, f32), at: (f32, f32)) -> bool {
    let (x, y, w, h) = bounds;
    at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h
}

fn set_flag(flag: Signal<bool>, next: bool) {
    if flag.peek() != next {
        flag.set(next);
    }
}
