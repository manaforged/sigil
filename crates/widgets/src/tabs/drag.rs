//! Drag-to-reorder support for the tabs widget.
//!
//! The drag model:
//!
//! - Each tab label installs `pointer_down` that records the visual index
//!   that the gesture started from into a shared [`DragState`].
//! - The tab-list container installs `pointer_move` + `pointer_up`. Move
//!   recomputes the visual position the pointer currently sits over and,
//!   if it crossed a sibling's midpoint, swaps the entry in the bound
//!   order signal so the host observes the new permutation. Up settles
//!   the drag and arms a "just dragged" latch so the next pointer-up on
//!   the source tab skips its activate hook.
//! - Movement of the order signal does NOT change `active` — original
//!   indices remain stable so content routing keeps tracking the same
//!   logical tab regardless of where it ended up visually.

use std::cell::Cell;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

/// Shared per-tabs-instance drag state. Each `tabs(...)` invocation
/// allocates one shared cell triple that lives for the program lifetime
/// — the cost is a single small leak per widget instance and saves us
/// from threading `Rc<RefCell<_>>` through every handler closure.
#[derive(Clone, Copy)]
pub(super) struct DragState {
    inner: &'static DragStateInner,
}

struct DragStateInner {
    source: Cell<Option<u16>>,
    current: Cell<Option<u16>>,
    just_dragged: Cell<bool>,
    /// Bound order signal — drag swaps write into it. Stored here so the
    /// per-tab pointer-move/up handlers can reach it without threading
    /// extra args through `build_tab`.
    order: Cell<Option<Signal<Vec<usize>>>>,
    count: Cell<u16>,
    /// Measured width of the drag-source tab, captured from the
    /// pointer-down event. ponytail: bucket math assumes every tab
    /// shares the source's width — exact for uniform labels, approximate
    /// for mixed-length ones; upgrade trigger is plumbing per-tab
    /// `LayoutResults` into the drag handlers.
    tab_width: Cell<f32>,
}

impl DragState {
    pub(super) fn new() -> Self {
        let inner = Box::leak(Box::new(DragStateInner {
            source: Cell::new(None),
            current: Cell::new(None),
            just_dragged: Cell::new(false),
            order: Cell::new(None),
            count: Cell::new(0),
            tab_width: Cell::new(0.0),
        }));
        Self { inner }
    }

    /// Bind the order signal + tab count for drag swaps. Called once at
    /// build time after the tab-list shape is known.
    pub(super) fn bind(self, order: Option<Signal<Vec<usize>>>, count: usize) {
        self.inner.order.set(order);
        self.inner
            .count
            .set(u16::try_from(count).unwrap_or(u16::MAX));
    }

    fn begin(self, visual_index: u16, width: f32) {
        self.inner.source.set(Some(visual_index));
        self.inner.current.set(Some(visual_index));
        self.inner.just_dragged.set(false);
        self.inner.tab_width.set(width);
    }

    fn tab_width(self) -> f32 {
        let width = self.inner.tab_width.get();
        // Defensive: a move without a recorded down width falls back to
        // a sane bucket rather than dividing the strip into zero-width
        // slots.
        if width > 1.0 { width } else { 80.0 }
    }

    const fn current(self) -> Option<u16> {
        self.inner.current.get()
    }

    fn set_current(self, visual_index: u16) {
        self.inner.current.set(Some(visual_index));
    }

    fn end(self) {
        let source = self.inner.source.take();
        let current = self.inner.current.take();
        if let (Some(start), Some(now)) = (source, current)
            && start != now
        {
            self.inner.just_dragged.set(true);
        }
    }

    /// Test-only: was a drag just completed? Consumes the latch.
    pub(super) const fn consume_just_dragged(self) -> bool {
        self.inner.just_dragged.replace(false)
    }

    const fn is_dragging(self) -> bool {
        self.inner.source.get().is_some()
    }

    const fn order(self) -> Option<Signal<Vec<usize>>> {
        self.inner.order.get()
    }

    const fn count(self) -> u16 {
        self.inner.count.get()
    }
}

/// Install all three pointer hooks (down/move/up) on a single tab so
/// the drag gesture lives entirely on the captured node.
///
/// The router only dispatches captured pointer events to the captured
/// node itself; it does not bubble to ancestors. Pointer-down captures
/// the pointer on this tab, so move + up must also be installed here
/// to fire under the captured-pointer path.
pub(super) fn install_label_drag_hooks(
    node: NodeBuilder,
    drag: DragState,
    visual_index: usize,
) -> NodeBuilder {
    let slot = u16::try_from(visual_index).unwrap_or(u16::MAX);
    node.pointer_down(move |event| {
        drag.begin(slot, event.target_size.width);
        event.capture_pointer();
    })
    .pointer_move(move |event| {
        if !drag.is_dragging() {
            return;
        }
        let Some(current) = drag.current() else {
            return;
        };
        // event.position is local to the captured tab (the source slot
        // while the pointer is still over it). Reconstruct the
        // tab-list-relative x by anchoring against the current visual
        // slot the drag is tracking.
        let tab_width = drag.tab_width();
        let tab_list_x = f32::from(current).mul_add(tab_width, event.position.x);
        let target = visual_index_for(tab_list_x, tab_width, drag.count());
        if target != current {
            apply_swap(drag.order(), current, target, usize::from(drag.count()));
            drag.set_current(target);
        }
    })
    .pointer_up(move |_event| {
        drag.end();
    })
}

fn visual_index_for(x: f32, tab_width: f32, count: u16) -> u16 {
    if count == 0 {
        return 0;
    }
    let max = count.saturating_sub(1);
    // Walk the [0, max] range searching for the bucket whose left edge
    // is the largest one still ≤ x. This sidesteps every f32→integer
    // cast clippy complains about; tab counts are bounded by u16 so the
    // linear scan is cheap (typical UI: < 20 tabs).
    let mut bucket: u16 = 0;
    let mut probe: u16 = 1;
    while probe <= max {
        let left = f32::from(probe) * tab_width;
        if x >= left {
            bucket = probe;
        } else {
            break;
        }
        probe = probe.saturating_add(1);
        if probe == u16::MAX {
            break;
        }
    }
    bucket.min(max)
}

fn apply_swap(order: Option<Signal<Vec<usize>>>, from: u16, to: u16, count: usize) {
    let Some(signal) = order else {
        return;
    };
    let mut next = signal.peek();
    if next.len() != count {
        // Out-of-shape signal — reset to identity before mutating so we
        // never write a permutation of the wrong length.
        next = (0..count).collect();
    }
    let from_idx = usize::from(from);
    let to_idx = usize::from(to);
    if from_idx < next.len() && to_idx < next.len() {
        next.swap(from_idx, to_idx);
        signal.set(next);
    }
}
