//! Deferred focus-request queue.
//!
//! Signal effects cannot hold a `&mut Cx` or reach the `EventRouter`,
//! so a widget that needs focus in response to a signal change pushes
//! the target through [`request_focus`] — a thread-local queue drained
//! by [`EventRouter::flush_focus_requests`], which [`EventRouter::route`]
//! calls before dispatching each event. This mirrors the
//! `sigil_kernel::queue_announcement` / `Cx::flush_announcements` seam:
//! same constraint (effects run without tree access), same shape.
//!
//! [`EventRouter::flush_focus_requests`]: crate::EventRouter::flush_focus_requests
//! [`EventRouter::route`]: crate::EventRouter::route

use std::cell::RefCell;

use sigil_kernel::NodeId;

thread_local! {
    static PENDING: RefCell<Vec<NodeId>> = const { RefCell::new(Vec::new()) };
}

/// Queue a focus request for `id`, applied by the router before the next
/// event dispatches.
///
/// Requests are validated at drain time against the same contract Tab
/// navigation enforces — focusable, laid out, and inside the active
/// focus scope — so a request naming a stale, hidden, or behind-a-modal
/// node is dropped rather than moving focus somewhere the keyboard
/// could not reach.
pub fn request_focus(id: NodeId) {
    PENDING.with(|cell| cell.borrow_mut().push(id));
}

/// Drain every queued request, leaving the queue empty.
pub(crate) fn take_focus_requests() -> Vec<NodeId> {
    PENDING.with(|cell| core::mem::take(&mut *cell.borrow_mut()))
}
