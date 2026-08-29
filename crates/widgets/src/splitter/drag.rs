use std::cell::{Cell, RefCell};

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

use super::builder::SplitterOrientation;

#[derive(Clone, Copy)]
pub(super) struct SplitterState {
    inner: &'static SplitterStateInner,
}

struct SplitterStateInner {
    orientation: SplitterOrientation,
    sizes: RefCell<Vec<f32>>,
    mins: RefCell<Vec<f32>>,
    maxs: RefCell<Vec<f32>>,
    grows: Vec<f32>,
    snap_points: RefCell<Vec<Vec<f32>>>,
    snap_threshold: Cell<f32>,
    sizes_bind: Cell<Option<Signal<Vec<f32>>>>,
    drag_anchor: RefCell<Option<DragAnchor>>,
}

struct DragAnchor {
    handle: usize,
    down_axis: f32,
    snapshot: Vec<f32>,
}

impl SplitterState {
    pub(super) fn new(
        orientation: SplitterOrientation,
        sizes: Vec<f32>,
        mins: Vec<f32>,
        maxs: Vec<f32>,
        grows: Vec<f32>,
    ) -> Self {
        let count = sizes.len();
        let handle_count = count.saturating_sub(1);
        let inner = Box::leak(Box::new(SplitterStateInner {
            orientation,
            sizes: RefCell::new(sizes),
            mins: RefCell::new(mins),
            maxs: RefCell::new(maxs),
            grows,
            snap_points: RefCell::new(vec![Vec::new(); handle_count]),
            snap_threshold: Cell::new(8.0),
            sizes_bind: Cell::new(None),
            drag_anchor: RefCell::new(None),
        }));
        Self { inner }
    }

    pub(super) const fn orientation(self) -> SplitterOrientation {
        self.inner.orientation
    }

    pub(super) fn size_at(self, pane: usize) -> f32 {
        self.inner.sizes.borrow().get(pane).copied().unwrap_or(0.0)
    }

    pub(super) fn set_snap(self, handle: usize, points: Vec<f32>) {
        if let Some(slot) = self.inner.snap_points.borrow_mut().get_mut(handle) {
            *slot = points;
        }
    }

    pub(super) fn set_snap_threshold(self, threshold: f32) {
        self.inner.snap_threshold.set(threshold.max(0.0));
    }

    pub(super) fn bind_sizes(self, signal: Signal<Vec<f32>>) {
        signal.set(self.inner.sizes.borrow().clone());
        self.inner.sizes_bind.set(Some(signal));
    }

    fn publish(self) {
        if let Some(signal) = self.inner.sizes_bind.get() {
            signal.set(self.inner.sizes.borrow().clone());
        }
    }

    fn begin(self, handle: usize, axis_position: f32) {
        let snapshot = self.inner.sizes.borrow().clone();
        *self.inner.drag_anchor.borrow_mut() = Some(DragAnchor {
            handle,
            down_axis: axis_position,
            snapshot,
        });
    }

    fn apply_delta(self, handle: usize, delta: f32) {
        let (snapshot, _down) = {
            let anchor = self.inner.drag_anchor.borrow();
            let Some(anchor) = anchor.as_ref() else {
                return;
            };
            if anchor.handle != handle {
                return;
            }
            (anchor.snapshot.clone(), anchor.down_axis)
        };
        let next = redistribute(
            &snapshot,
            handle,
            delta,
            &PaneBounds {
                mins: &self.inner.mins.borrow(),
                maxs: &self.inner.maxs.borrow(),
                grows: &self.inner.grows,
            },
        );
        *self.inner.sizes.borrow_mut() = next;
        self.publish();
    }

    fn end(self) {
        let anchor = self.inner.drag_anchor.borrow_mut().take();
        let Some(anchor) = anchor else {
            return;
        };
        let snap_points = self.inner.snap_points.borrow();
        let Some(points) = snap_points.get(anchor.handle) else {
            return;
        };
        if points.is_empty() {
            return;
        }
        let threshold = self.inner.snap_threshold.get();
        let current_split = {
            let sizes = self.inner.sizes.borrow();
            cumulative_split(&sizes, anchor.handle)
        };
        let Some(target) = nearest_snap(points, current_split, threshold) else {
            return;
        };
        let delta = target - current_split;
        let next = redistribute(
            &self.inner.sizes.borrow().clone(),
            anchor.handle,
            delta,
            &PaneBounds {
                mins: &self.inner.mins.borrow(),
                maxs: &self.inner.maxs.borrow(),
                grows: &self.inner.grows,
            },
        );
        *self.inner.sizes.borrow_mut() = next;
        self.publish();
    }
}

struct PaneBounds<'a> {
    mins: &'a [f32],
    maxs: &'a [f32],
    grows: &'a [f32],
}

fn redistribute(snapshot: &[f32], handle: usize, delta: f32, bounds: &PaneBounds<'_>) -> Vec<f32> {
    let mut out = snapshot.to_vec();
    let left_grows = bounds.grows.get(handle).copied().unwrap_or(0.0) > 0.0;
    let right_grows = bounds.grows.get(handle + 1).copied().unwrap_or(0.0) > 0.0;
    if left_grows != right_grows {
        let fixed = if right_grows { handle } else { handle + 1 };
        let signed = if right_grows { delta } else { -delta };
        let base = snapshot.get(fixed).copied().unwrap_or(0.0);
        let min = bounds.mins.get(fixed).copied().unwrap_or(0.0);
        let max = bounds.maxs.get(fixed).copied().unwrap_or(f32::INFINITY);
        if let Some(slot) = out.get_mut(fixed) {
            *slot = (base + signed).clamp(min, max);
        }
        return out;
    }
    let Some(&grow_base) = snapshot.get(handle) else {
        return out;
    };
    let Some(&shrink_base) = snapshot.get(handle + 1) else {
        return out;
    };
    let grow_min = bounds.mins.get(handle).copied().unwrap_or(0.0);
    let grow_max = bounds.maxs.get(handle).copied().unwrap_or(f32::INFINITY);
    let shrink_min = bounds.mins.get(handle + 1).copied().unwrap_or(0.0);
    let shrink_max = bounds
        .maxs
        .get(handle + 1)
        .copied()
        .unwrap_or(f32::INFINITY);

    let max_delta = (grow_max - grow_base).min(shrink_base - shrink_min);
    let min_delta = (grow_min - grow_base).max(shrink_base - shrink_max);
    let applied = delta.clamp(min_delta, max_delta);

    if let Some(slot) = out.get_mut(handle) {
        *slot = grow_base + applied;
    }
    if let Some(slot) = out.get_mut(handle + 1) {
        *slot = shrink_base - applied;
    }
    out
}

fn cumulative_split(sizes: &[f32], handle: usize) -> f32 {
    sizes.iter().take(handle + 1).copied().sum()
}

fn nearest_snap(points: &[f32], current: f32, threshold: f32) -> Option<f32> {
    let mut best: Option<(f32, f32)> = None;
    for &p in points {
        let distance = (p - current).abs();
        if distance > threshold {
            continue;
        }
        match best {
            None => best = Some((p, distance)),
            Some((_, d)) if distance < d => best = Some((p, distance)),
            _ => {}
        }
    }
    best.map(|(p, _)| p)
}

pub(super) fn install_handle_hooks(
    node: NodeBuilder,
    state: SplitterState,
    handle_index: usize,
) -> NodeBuilder {
    node.pointer_down(move |event| {
        let axis = axis_of(state.orientation(), event.position.x, event.position.y);
        state.begin(handle_index, axis);
        event.capture_pointer();
        event.stop_propagation();
    })
    .pointer_move(move |event| {
        if !event.is_captured() {
            return;
        }
        let anchor_axis = {
            let anchor = state.inner.drag_anchor.borrow();
            anchor.as_ref().map(|a| a.down_axis)
        };
        let Some(anchor_axis) = anchor_axis else {
            return;
        };
        let current_axis = axis_of(state.orientation(), event.position.x, event.position.y);
        let delta = current_axis - anchor_axis;
        state.apply_delta(handle_index, delta);
        event.stop_propagation();
    })
    .pointer_up(move |event| {
        state.end();
        event.stop_propagation();
    })
}

const fn axis_of(orientation: SplitterOrientation, x: f32, y: f32) -> f32 {
    match orientation {
        SplitterOrientation::Horizontal => x,
        SplitterOrientation::Vertical => y,
    }
}

#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
