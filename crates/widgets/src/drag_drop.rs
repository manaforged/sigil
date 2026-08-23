//! `drag_drop` — payload-carrying, widget-to-widget drag sessions.
//!
//! The missing middle between [`crate::axis_drag`] (position-only handle
//! drags) and the OS file drop: a [`DragState<T>`] session carries a
//! typed payload from a `.drag_source(...)` node to a `.drop_target(...)`
//! node, with a floating [`drag_ghost`] chip following the pointer.
//!
//! ## Session model (signal-first)
//!
//! * Left `pointer_down` on a source ARMS a candidate — no capture, no
//!   stop-propagation, so click-to-select and right-click menus keep
//!   working.
//! * A move past [`DRAG_THRESHOLD_PX`] begins the session: payload +
//!   position publish into the state's signals and the source captures
//!   the pointer. Capture binds to the REQUESTING node, not the hit
//!   leaf, so the session survives leaving the source's children.
//! * Captured moves track `position` and hit-test registered targets
//!   against their layout-published bounds; the hovered accepting target
//!   gets [`DROP_TARGET_CLASS`] reactively.
//! * `pointer_up` over an accepting target fires `on_drop(payload)`;
//!   anywhere else cancels cleanly.
//!
//! ## Ceilings
//!
//! * No autoscroll near scroll-container edges. Upgrade trigger: a
//!   consumer dragging through long lists.
//! * Overlapping targets: the last-built (topmost) containing target
//!   wins; a rejecting target has no fall-through to one underneath.
//! * Target bounds read last frame's published layout (router contract).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil_events::prelude::{MouseButton, MouseButtons, NodeEventsExt, PointerEvent};
use sigil_kernel::{NodeBuilder, container};
use sigil_layout::LayoutReactiveExt;
use sigil_reactive::{Signal, derived, signal};
use sigil_style::NodeStyleExt;

/// Pointer travel (logical px) before a press becomes a drag.
pub const DRAG_THRESHOLD_PX: f32 = 4.0;

/// Class toggled on the hovered drop target while it accepts the payload.
pub const DROP_TARGET_CLASS: &str = "bg-accent-4";

const GHOST_CLASS: &str = "rounded-md px-2 py-1 bg-accent-3 text-accent-12 border border-accent-6";
/// Chip offset from the pointer so it doesn't sit under the cursor.
const GHOST_OFFSET: (f32, f32) = (12.0, 10.0);

type Armed = Rc<Cell<Option<(f32, f32)>>>;
type Accepts<T> = Rc<dyn Fn(&T) -> bool>;
type OnDrop<T> = Rc<dyn Fn(T)>;

/// Shared handle to a drag session channel with payload type `T`.
///
/// Create one per drag DOMAIN (e.g. "tasks on the group rail"), share it
/// between every source and target of that domain, and mount one
/// [`drag_ghost`] per domain near the app root. Clone handles freely —
/// all clones talk to the same session.
///
/// # Panics
///
/// Creating outside [`Runtime::scope`](sigil_reactive::Runtime::scope)
/// panics on the next signal read (the signals are scope-owned).
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

/// Deregisters the drop-target entry when the node leaves the tree
/// (rebuild, windowed scroll-off), so stale bounds never hit-test.
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
    /// Create an idle drag-session channel.
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

    /// Whether a drag session is currently active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active.peek()
    }

    /// Current pointer position (window coordinates) of the session.
    #[must_use]
    pub fn position(&self) -> (f32, f32) {
        self.position.peek()
    }

    /// The payload being dragged, while a session is active.
    #[must_use]
    pub fn payload(&self) -> Option<T> {
        self.payload.peek()
    }

    /// End the active session without dropping. Safe when idle.
    pub fn cancel(&self) {
        self.end();
    }

    fn begin(&self, payload: T, at: (f32, f32)) {
        // `set_always`: payloads aren't required to be `PartialEq`.
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
        // Invoke AFTER the registry borrow is released and the session has
        // ended: `on_drop` mutates app state and may rebuild targets inline.
        if let Some((on_drop, payload)) = drop {
            on_drop(payload);
        }
    }

    fn end(&self) {
        self.active.set(false);
        // Payload cleared first, so this hover pass clears every target.
        self.payload.set_always(None);
        self.update_hover((0.0, 0.0));
    }

    /// The topmost (last-built) target under `at` that accepts `payload`.
    fn accepting_drop(&self, at: (f32, f32), payload: &T) -> Option<OnDrop<T>> {
        self.targets
            .borrow()
            .targets
            .iter()
            .rev()
            .find(|target| contains(target.bounds.peek(), at) && (target.accepts)(payload))
            .map(|target| Rc::clone(&target.on_drop))
    }

    /// Highlight exactly the topmost containing target that accepts the
    /// payload; clear every other target.
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

/// The drag-drop mixin surface on [`NodeBuilder`].
pub trait DragDropExt<T: Clone + 'static> {
    /// Make this node a drag source carrying `payload` (cloned when the
    /// session begins). A plain click never starts a drag: the press must
    /// travel past [`DRAG_THRESHOLD_PX`] first.
    #[must_use]
    fn drag_source(self, state: DragState<T>, payload: T) -> Self;

    /// Make this node a drop target. While a session is active and the
    /// pointer is over this node, `accepts` gates the highlight; on
    /// pointer-up an accepting target fires `on_drop(payload)`.
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

/// The floating chip that follows the pointer during a session.
///
/// Mount once per [`DragState`] domain as the LAST child of the app-root
/// container (position is absolute against the parent, and later children
/// paint on top). `render` builds the chip content from the payload —
/// e.g. `move |task| text(task.name.clone())`. The node is non-hittable
/// and empty while idle, so it never intercepts pointer input.
#[must_use]
pub fn drag_ghost<T, N>(state: DragState<T>, render: impl Fn(&T) -> N + 'static) -> NodeBuilder
where
    T: Clone + 'static,
    N: sigil_kernel::IntoNode + 'static,
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
    N: sigil_kernel::IntoNode + 'static,
{
    if !state.active.get() {
        return Vec::new();
    }
    let Some(payload) = state.payload.get() else {
        return Vec::new();
    };
    vec![container().class(GHOST_CLASS).child(render(&payload))]
}

/// Arm a drag candidate on left-press; never captures or stops
/// propagation, so plain clicks and right-click menus pass through.
fn arm(armed: Armed) -> impl Fn(&mut PointerEvent) + 'static {
    move |event| {
        if event.button == Some(MouseButton::Left) {
            armed.set(Some((event.window_position.x, event.window_position.y)));
        }
    }
}

/// Drive the session: threshold-crossing begins it (and captures the
/// pointer onto the source); captured moves track position + hover.
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
        // The button went away without an Up reaching us (window blur,
        // OS cancel) — heal instead of sticking the session on.
        state.cancel();
        return;
    }
    state.track(at);
    event.stop_propagation();
}

/// Release: disarm the candidate; settle (drop or cancel) only when a
/// session is active — plain clicks pass through untouched.
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
