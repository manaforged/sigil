use std::cell::RefCell;

use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};

use crate::widgets::drag::{Axis, axis_drag};
use crate::widgets::orientation::Orientation;

#[derive(Clone, Copy)]
pub(super) struct SplitterState {
    inner: &'static SplitterStateInner,
}

struct SplitterStateInner {
    orientation: Orientation,
    sizes: RefCell<Vec<f32>>,
    mins: Vec<f32>,
    maxs: Vec<f32>,
    grows: Vec<f32>,
    snap_points: Vec<Vec<f32>>,
    snap_threshold: f32,
    sizes_bind: Option<Signal<Vec<f32>>>,
    revision: Signal<u64>,
}

pub(super) struct PaneLimits {
    pub(super) sizes: Vec<f32>,
    pub(super) mins: Vec<f32>,
    pub(super) maxs: Vec<f32>,
    pub(super) grows: Vec<f32>,
}

impl SplitterState {
    pub(super) fn new(
        orientation: Orientation,
        limits: PaneLimits,
        snaps: Vec<(usize, Vec<f32>)>,
        snap_threshold: f32,
        sizes_bind: Option<Signal<Vec<f32>>>,
    ) -> Self {
        let mut snap_points = vec![Vec::new(); limits.sizes.len().saturating_sub(1)];
        for (handle, points) in snaps {
            if let Some(slot) = snap_points.get_mut(handle) {
                *slot = points;
            }
        }
        if let Some(signal) = sizes_bind {
            signal.set(limits.sizes.clone());
        }
        let inner = Box::leak(Box::new(SplitterStateInner {
            orientation,
            sizes: RefCell::new(limits.sizes),
            mins: limits.mins,
            maxs: limits.maxs,
            grows: limits.grows,
            snap_points,
            snap_threshold: snap_threshold.max(0.0),
            sizes_bind,
            revision: signal(0),
        }));
        Self { inner }
    }

    pub(super) const fn orientation(self) -> Orientation {
        self.inner.orientation
    }

    pub(super) fn size_at(self, pane: usize) -> f32 {
        self.inner.sizes.borrow().get(pane).copied().unwrap_or(0.0)
    }

    pub(super) fn tracked_size(self, pane: usize) -> f32 {
        self.inner.revision.track();
        self.size_at(pane)
    }

    pub(super) fn bounds(self, handle: usize) -> (f32, f32) {
        let min = self.inner.mins.get(handle).copied().unwrap_or(0.0);
        let max = self
            .inner
            .maxs
            .get(handle)
            .copied()
            .unwrap_or(f32::INFINITY);
        let span = self.size_at(handle) + self.size_at(handle + 1);
        (min, if max.is_finite() { max } else { span.max(min) })
    }

    pub(super) fn nudge(self, handle: usize, delta: f32) {
        let snapshot = self.inner.sizes.borrow().clone();
        self.resize(&snapshot, handle, delta);
    }

    fn resize(self, snapshot: &[f32], handle: usize, delta: f32) {
        let bounds = PaneBounds {
            mins: &self.inner.mins,
            maxs: &self.inner.maxs,
            grows: &self.inner.grows,
        };
        let next = redistribute(snapshot, handle, delta, &bounds);
        *self.inner.sizes.borrow_mut() = next;
        self.inner
            .revision
            .update(|value| *value = value.wrapping_add(1));
        if let Some(signal) = self.inner.sizes_bind {
            signal.set(self.inner.sizes.borrow().clone());
        }
    }

    fn snap(self, handle: usize) {
        let Some(points) = self.inner.snap_points.get(handle) else {
            return;
        };
        let sizes = self.inner.sizes.borrow().clone();
        let current = cumulative_split(&sizes, handle);
        if let Some(target) = nearest_snap(points, current, self.inner.snap_threshold) {
            self.resize(&sizes, handle, target - current);
        }
    }
}

struct PaneBounds<'a> {
    mins: &'a [f32],
    maxs: &'a [f32],
    grows: &'a [f32],
}

fn redistribute(snapshot: &[f32], handle: usize, delta: f32, bounds: &PaneBounds<'_>) -> Vec<f32> {
    let mut out = snapshot.to_vec();
    let grows = |i: usize| bounds.grows.get(i).copied().unwrap_or(0.0) > 0.0;
    let min = |i: usize| bounds.mins.get(i).copied().unwrap_or(0.0);
    let max = |i: usize| bounds.maxs.get(i).copied().unwrap_or(f32::INFINITY);
    let (left_grows, right_grows) = (grows(handle), grows(handle + 1));
    if left_grows != right_grows {
        let (fixed, signed) = if right_grows {
            (handle, delta)
        } else {
            (handle + 1, -delta)
        };
        let base = snapshot.get(fixed).copied().unwrap_or(0.0);
        if let Some(slot) = out.get_mut(fixed) {
            *slot = (base + signed).clamp(min(fixed), max(fixed));
        }
        return out;
    }
    let (Some(&grow_base), Some(&shrink_base)) = (snapshot.get(handle), snapshot.get(handle + 1))
    else {
        return out;
    };
    let max_delta = (max(handle) - grow_base).min(shrink_base - min(handle + 1));
    let min_delta = (min(handle) - grow_base).max(shrink_base - max(handle + 1));
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
    points
        .iter()
        .copied()
        .filter(|p| (p - current).abs() <= threshold)
        .min_by(|a, b| (a - current).abs().total_cmp(&(b - current).abs()))
}

pub(super) fn install_handle_hooks(
    node: NodeBuilder,
    state: SplitterState,
    handle_index: usize,
) -> NodeBuilder {
    let axis = match state.orientation() {
        Orientation::Horizontal => Axis::X,
        Orientation::Vertical => Axis::Y,
    };
    axis_drag(
        node,
        axis,
        move || state.inner.sizes.borrow().clone(),
        move |snapshot, delta| state.resize(&snapshot, handle_index, delta),
        move || state.snap(handle_index),
    )
}

#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
