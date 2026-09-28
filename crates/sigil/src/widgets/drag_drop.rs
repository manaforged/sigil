use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::events::prelude::{MouseButton, MouseButtons, NodeEventsExt, PointerEvent};
use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutReactiveExt;
use crate::reactive::{Signal, derived, signal};
use crate::style::{BORDER_WIDTH, NodeStyleExt, Radius, StyleAtom, Token};

pub const DRAG_THRESHOLD_PX: f32 = 4.0;

pub const DROP_TARGET_CLASS: &str = "bg-accent-4";

const GHOST_OFFSET: (f32, f32) = (12.0, 10.0);

type Armed = Rc<Cell<Option<(f32, f32)>>>;
type Accepts<T> = Rc<dyn Fn(&T) -> bool>;
type OnDrop<T> = Rc<dyn Fn(T)>;

pub struct DragState<T: 'static> {
    active: Signal<bool>,
    position: Signal<(f32, f32)>,
    payload: Signal<Option<T>>,
    targets: Rc<RefCell<Registry<T>>>,
}

struct Registry<T: 'static> {
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
    registry: Rc<RefCell<Registry<T>>>,
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
            active: self.active,
            position: self.position,
            payload: self.payload,
            targets: Rc::clone(&self.targets),
        }
    }
}

impl<T: Clone + 'static> DragState<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: signal(false),
            position: signal((0.0, 0.0)),
            payload: signal(None),
            targets: Rc::new(RefCell::new(Registry {
                targets: Vec::new(),
                next_id: 0,
            })),
        }
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active.try_peek().unwrap_or(false)
    }

    #[must_use]
    pub fn position(&self) -> (f32, f32) {
        self.position.try_peek().unwrap_or_default()
    }

    #[must_use]
    pub fn payload(&self) -> Option<T> {
        self.payload.try_peek().flatten()
    }

    pub fn cancel(&self) {
        self.end();
    }

    fn begin(&self, payload: T, at: (f32, f32)) {
        self.payload.set_always(Some(payload));
        self.position.set(at);
        self.active.set(true);
        self.update_hover(at);
    }

    fn track(&self, at: (f32, f32)) {
        self.position.set(at);
        self.update_hover(at);
    }

    fn settle(&self, at: (f32, f32)) {
        let drop = self
            .payload()
            .and_then(|payload| self.accepting_drop(at, &payload).map(|cb| (cb, payload)));
        self.end();
        if let Some((on_drop, payload)) = drop {
            on_drop(payload);
        }
    }

    fn end(&self) {
        self.active.set(false);
        self.payload.set_always(None);
        self.update_hover((0.0, 0.0));
    }

    fn accepting_drop(&self, at: (f32, f32), payload: &T) -> Option<OnDrop<T>> {
        self.targets
            .borrow()
            .targets
            .iter()
            .rev()
            .find(|target| contains(target.bounds.peek(), at) && (target.accepts)(payload))
            .map(|target| Rc::clone(&target.on_drop))
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

pub trait DragDropExt<T: Clone + 'static> {
    #[must_use]
    fn drag_source(self, state: DragState<T>, payload: T) -> Self;

    #[must_use]
    fn drop_target(
        self,
        state: DragState<T>,
        accepts: impl Fn(&T) -> bool + 'static,
        on_drop: impl Fn(T) + 'static,
    ) -> Self;
}

impl<T: Clone + 'static> DragDropExt<T> for NodeBuilder {
    fn drag_source(self, state: DragState<T>, payload: T) -> Self {
        let armed: Armed = Rc::new(Cell::new(None));
        self.pointer_down(arm(Rc::clone(&armed)))
            .pointer_move(drive(state.clone(), Rc::clone(&armed), payload))
            .pointer_up(release(state, armed))
    }

    fn drop_target(
        self,
        state: DragState<T>,
        accepts: impl Fn(&T) -> bool + 'static,
        on_drop: impl Fn(T) + 'static,
    ) -> Self {
        let bounds = signal((0.0, 0.0, 0.0, 0.0));
        let highlight = signal(false);
        let guard = state.register(bounds, highlight, Rc::new(accepts), Rc::new(on_drop));
        self.bounds_reactive(bounds)
            .class_when(highlight, DROP_TARGET_CLASS)
            .mutate(move |node| {
                let _ = node.attach(guard);
            })
    }
}

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
    if !state.active.get() {
        return Vec::new();
    }
    let Some(payload) = state.payload.get() else {
        return Vec::new();
    };
    vec![
        container()
            .class("px-8 py-4")
            .class([
                Radius::Md.atom(),
                Token::Accent3.bg(),
                Token::Accent12.text(),
                StyleAtom::BorderWidth(BORDER_WIDTH),
                Token::Accent6.border(),
            ])
            .child(render(&payload)),
    ]
}

fn arm(armed: Armed) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        if event.button == Some(MouseButton::Left) {
            armed.set(Some((event.window_position.x, event.window_position.y)));
        }
    }
}

fn drive<T: Clone + 'static>(
    state: DragState<T>,
    armed: Armed,
    payload: T,
) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        let at = (event.window_position.x, event.window_position.y);
        if state.is_active() {
            drive_session(&state, event, at);
            return;
        }
        let Some(down) = armed.get() else {
            return;
        };
        if travelled(down, at) >= DRAG_THRESHOLD_PX {
            armed.set(None);
            state.begin(payload.clone(), at);
            event.capture_pointer();
            event.stop_propagation();
        }
    }
}

fn drive_session<T: Clone + 'static>(
    state: &DragState<T>,
    event: &mut PointerEvent,
    at: (f32, f32),
) {
    if !event.buttons.contains(MouseButtons::LEFT) {
        state.cancel();
        return;
    }
    state.track(at);
    event.stop_propagation();
}

fn release<T: Clone + 'static>(
    state: DragState<T>,
    armed: Armed,
) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        armed.set(None);
        if state.is_active() {
            state.settle((event.window_position.x, event.window_position.y));
            event.stop_propagation();
        }
    }
}

fn travelled(down: (f32, f32), at: (f32, f32)) -> f32 {
    let (dx, dy) = (at.0 - down.0, at.1 - down.1);
    dx.hypot(dy)
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
