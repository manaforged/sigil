//! `axis_drag` — the reusable capture-on-handle drag primitive.
//!
//! Extracted from `splitter/drag.rs` (5/5 Wave 2, the reviewer gate #6) so any
//! widget with draggable affordances — splitter handles, table column
//! resize, gauge scrubbing — gets the same proven flow instead of a
//! bespoke copy:
//!
//! 1. `pointer_down` snapshots the caller's state and the pointer's axis
//!    position, then captures the pointer (the router now delivers
//!    move/up to this node).
//! 2. `pointer_move` applies `current_axis − down_axis` against the
//!    stable snapshot — never compounding against live state.
//! 3. `pointer_up` settles the drag and releases via `on_end`.
//!
//! ```ignore
//! axis_drag(handle, Axis::X,
//!     move || widths.peek(),
//!     move |snapshot, delta| {
//!         let mut next = snapshot.clone();
//!         next[col] = (next[col] + delta).max(MIN_COL_WIDTH);
//!         widths.set(next);
//!     },
//!     || {},
//! );
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::NodeBuilder;

/// Which pointer axis a drag tracks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    /// Horizontal (x position).
    X,
    /// Vertical (y position).
    Y,
}

const fn axis_of(axis: Axis, x: f32, y: f32) -> f32 {
    match axis {
        Axis::X => x,
        Axis::Y => y,
    }
}

/// Install capture-on-handle drag handlers on `node`.
///
/// * `snapshot` runs once at pointer-down; the returned state is cloned
///   into the drag session.
/// * `on_delta(snapshot, delta)` fires per captured move with the
///   pointer's axis delta since pointer-down.
/// * `on_end` fires once at pointer-up.
///
/// Handlers stop propagation at each stage so siblings don't also act
/// on the drag; hover channels are untouched.
pub fn axis_drag<S>(
    node: NodeBuilder,
    axis: Axis,
    snapshot: impl Fn() -> S + 'static,
    on_delta: impl Fn(S, f32) + 'static,
    on_end: impl Fn() + 'static,
) -> NodeBuilder
where
    S: Clone + 'static,
{
    let session = Rc::new(RefCell::new(None::<(f32, S)>));

    let begin_state = Rc::clone(&session);
    let move_state = Rc::clone(&session);

    node.pointer_down(move |event| {
        let axis_pos = axis_of(axis, event.position.x, event.position.y);
        *begin_state.borrow_mut() = Some((axis_pos, snapshot()));
        event.capture_pointer();
        event.stop_propagation();
    })
    .pointer_move(move |event| {
        if !event.is_captured() {
            return;
        }
        let Some((down_axis, base)) = move_state.borrow().clone() else {
            return;
        };
        let current = axis_of(axis, event.position.x, event.position.y);
        on_delta(base, current - down_axis);
        event.stop_propagation();
    })
    .pointer_up(move |event| {
        let was_dragging = session.borrow().is_some();
        session.borrow_mut().take();
        if was_dragging {
            on_end();
        }
        event.stop_propagation();
    })
}
