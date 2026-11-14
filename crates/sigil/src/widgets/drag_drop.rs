use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

use crate::events::{DropEvent, MouseButtons, PointerEvent};
use crate::geometry::{Point, Rect};
use crate::kernel::{NodeBuilder, container};
use crate::reactive::{Signal, memo, signal};
use crate::style::{BORDER_WIDTH, Radius, StyleAtom, Tone};
use crate::widgets::Widget;
use crate::widgets::interaction::{is_cancel, is_primary};

const DROP_TARGET_CLASS: &str = "bg-accent-4";

const GHOST_OFFSET: Point = Point::new(12.0, 10.0);

type Armed<T> = Rc<RefCell<Option<(Point, T)>>>;
type Accepts<T> = Rc<dyn Fn(&T) -> bool>;
type OnDrop<T> = Rc<dyn Fn(DropEvent<T>)>;
type HoverCursorSlot = Rc<Cell<Option<Signal<Option<Point>>>>>;

/// Shared state of one drag interaction.
pub struct DragState<T: 'static> {
    position: Signal<Point>,
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
    bounds: Signal<Rect>,
    highlight: Signal<bool>,
    hover_cursor: HoverCursorSlot,
    accepts: Accepts<T>,
    on_drop: OnDrop<T>,
}

impl<T: 'static> Clone for DropTarget<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            bounds: self.bounds,
            highlight: self.highlight,
            hover_cursor: Rc::clone(&self.hover_cursor),
            accepts: Rc::clone(&self.accepts),
            on_drop: Rc::clone(&self.on_drop),
        }
    }
}

struct DropStateEntry<T: 'static> {
    registry: *const RefCell<DropRegistry<T>>,
    hover_cursor: Option<HoverCursorSlot>,
    cursor: Option<Signal<Option<Point>>>,
}

struct DropNodeExt<T: 'static> {
    entries: Vec<DropStateEntry<T>>,
    guards: Vec<TargetGuard<T>>,
    marker: PhantomData<fn() -> T>,
}

impl<T: 'static> DropNodeExt<T> {
    fn entry_mut(&mut self, registry: *const RefCell<DropRegistry<T>>) -> &mut DropStateEntry<T> {
        let idx = self
            .entries
            .iter()
            .position(|entry| core::ptr::eq(entry.registry, registry))
            .unwrap_or_else(|| {
                self.entries.push(DropStateEntry {
                    registry,
                    hover_cursor: None,
                    cursor: None,
                });
                self.entries.len() - 1
            });
        self.entries.get_mut(idx).unwrap_or_else(|| unreachable!())
    }
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
    /// Creates a drag state with no drag in progress.
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: signal(Point::ZERO),
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

    /// The last pointer position of a drag, in window coordinates.
    #[must_use]
    pub fn position(&self) -> Point {
        self.position.try_peek().unwrap_or_default()
    }

    /// The value that the user drags; `None` when no drag is in progress.
    #[must_use]
    pub fn payload(&self) -> Option<T> {
        self.payload.try_peek().flatten()
    }

    /// Cancels the current drag.
    pub fn cancel(&self) {
        self.end();
    }

    fn begin(&self, payload: T, at: Point) {
        self.payload.set_always(Some(payload));
        self.position.set(at);
        self.update_hover(at);
    }

    fn snapshot(&self) -> Vec<DropTarget<T>> {
        self.targets
            .borrow()
            .targets
            .iter()
            .rev()
            .cloned()
            .collect()
    }

    fn settle(&self, at: Point) {
        let drop = self.payload().and_then(|payload| {
            self.snapshot()
                .into_iter()
                .find(|target| contains(target.bounds.peek(), at) && (target.accepts)(&payload))
                .map(|target| {
                    let bounds = target.bounds.peek();
                    (
                        target.on_drop,
                        DropEvent::new(payload, at, local_point(bounds, at), bounds),
                    )
                })
        });
        self.end();
        if let Some((on_drop, event)) = drop {
            on_drop(event);
        }
    }

    fn end(&self) {
        self.payload.set_always(None);
        self.update_hover(Point::ZERO);
    }

    fn update_hover(&self, at: Point) {
        let payload = self.payload();
        let mut claimed = false;
        for target in self.snapshot() {
            let bounds = target.bounds.peek();
            let hover = !claimed
                && payload
                    .as_ref()
                    .is_some_and(|p| contains(bounds, at) && (target.accepts)(p));
            claimed |= hover;
            let custom = target.hover_cursor.get();
            let _ = target.highlight.try_set(hover && custom.is_none());
            if let Some(cursor) = custom {
                let _ = cursor.try_set(hover.then(|| local_point(bounds, at)));
            }
        }
    }

    fn register(&self, mut target: DropTarget<T>) -> TargetGuard<T> {
        let mut registry = self.targets.borrow_mut();
        let id = registry.next_id;
        registry.next_id += 1;
        target.id = id;
        registry.targets.push(target);
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
    on_drop: impl Fn(DropEvent<T>) + 'static,
) -> NodeBuilder {
    let bounds = signal(Rect::default());
    let highlight = signal(false);
    let hover_cursor = Rc::new(Cell::new(None));
    let registry_ptr = Rc::as_ptr(&state.targets);
    let guard = state.register(DropTarget {
        id: 0,
        bounds,
        highlight,
        hover_cursor: Rc::clone(&hover_cursor),
        accepts: Rc::new(accepts),
        on_drop: Rc::new(on_drop),
    });
    node.pointer_move(drive(state.clone()))
        .pointer_up(release(state))
        .track_bounds(bounds)
        .class_when(highlight, DROP_TARGET_CLASS)
        .mutate(move |node| {
            let ext = node_drop_ext::<T>(node);
            let entry = ext.entry_mut(registry_ptr);
            if let Some(cursor) = entry.cursor {
                hover_cursor.set(Some(cursor));
            }
            entry.hover_cursor = Some(hover_cursor);
            ext.guards.push(guard);
        })
}

pub(crate) fn hover<T: Clone + 'static>(
    node: NodeBuilder,
    state: &DragState<T>,
    local_cursor: Signal<Option<Point>>,
) -> NodeBuilder {
    let registry_ptr = Rc::as_ptr(&state.targets);
    node.mutate(move |node| {
        let entry = node_drop_ext::<T>(node).entry_mut(registry_ptr);
        entry.cursor = Some(local_cursor);
        if let Some(slot) = &entry.hover_cursor {
            slot.set(Some(local_cursor));
        }
    })
}

fn node_drop_ext<T: 'static>(node: &mut crate::kernel::Node) -> &mut DropNodeExt<T> {
    if node.extension::<DropNodeExt<T>>().is_none() {
        let _ = node.attach(DropNodeExt::<T> {
            entries: Vec::new(),
            guards: Vec::new(),
            marker: PhantomData,
        });
    }
    node.extension_mut::<DropNodeExt<T>>()
        .unwrap_or_else(|| unreachable!())
}

/// Returns a node that shows `render` of the dragged payload beside the pointer, and nothing when no drag is active.
pub fn drag_ghost<T, N>(state: DragState<T>, render: impl Fn(&T) -> N + 'static) -> NodeBuilder
where
    T: Clone + 'static,
    N: crate::kernel::IntoNode + 'static,
{
    let pos_state = state.clone();
    let pos = memo(move || {
        let at = pos_state.position.get();
        Point::new(at.x + GHOST_OFFSET.x, at.y + GHOST_OFFSET.y)
    });
    container()
        .hittable(false)
        .offset(pos)
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
                Tone::Primary.step(3).background(),
                Tone::Primary.step(12).text(),
                StyleAtom::BorderWidth(BORDER_WIDTH),
                Tone::Primary.step(6).border(),
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
            state
                .armed
                .replace(Some((event.window_position, payload.clone())));
            event.capture_pointer();
        }
    }
}

fn drive<T: Clone + 'static>(state: DragState<T>) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        let at = event.window_position;
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
            .filter(|(down, _)| event.passed_drag_threshold(*down))
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
            state.settle(event.window_position);
        }
        event.stop_propagation();
    }
}

fn local_point(bounds: Rect, at: Point) -> Point {
    Point::new(at.x - bounds.origin.x, at.y - bounds.origin.y)
}

fn contains(bounds: Rect, at: Point) -> bool {
    at.x >= bounds.origin.x
        && at.x < bounds.origin.x + bounds.width
        && at.y >= bounds.origin.y
        && at.y < bounds.origin.y + bounds.height
}
