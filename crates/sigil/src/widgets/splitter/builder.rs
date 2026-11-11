use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::style::{BORDER_WIDTH, Paint, Style, StyleAtom, Token};

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::caller_style;
use super::drag::{Binds, PaneLimits, SplitterState, install_handle_hooks};
use super::keys::install_handle_keys;
use super::pane::{build_pane, shares_free_space};
use super::pane_builder::PaneBuilder;
use crate::layout::Orientation;
use crate::style::States;
use crate::widgets::{Layout, Widget};

pub(in crate::widgets) const HANDLE_PX: f32 = 6.0;

static RULE: &[StyleAtom] = &[Token::Border.background()];

pub(super) struct PaneEntry {
    content: Box<dyn IntoNode>,
    pub(super) initial_size: f32,
    pub(super) min: f32,
    pub(super) max: f32,
    pub(super) grow: f32,
    pub(super) layout: LayoutOverrides,
    pub(super) style: Style,
}

impl PaneEntry {
    fn new(node: impl IntoNode + 'static, initial_size: f32) -> Self {
        Self {
            content: Box::new(node),
            initial_size: initial_size.max(0.0),
            min: 0.0,
            max: f32::INFINITY,
            grow: 0.0,
            layout: LayoutOverrides::new(),
            style: Style::new(),
        }
    }
}

/// Builder of a splitter.
#[must_use]
pub struct SplitterBuilder {
    pub(super) orientation: Orientation,
    pub(super) panes: Vec<PaneEntry>,
    sizes_bind: Option<crate::reactive::Signal<Vec<f32>>>,
    settle_bind: Option<crate::reactive::Signal<Vec<f32>>>,
    proportional: bool,
    snap_points: Vec<(usize, Vec<f32>)>,
    snap_threshold: f32,
    pub(super) layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

impl core::fmt::Debug for SplitterBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SplitterBuilder")
            .field("orientation", &self.orientation)
            .field("sizes_bind", &self.sizes_bind)
            .field("proportional", &self.proportional)
            .field("snap_threshold", &self.snap_threshold)
            .field("layout_overrides", &self.layout_overrides)
            .field("style_accumulator", &self.style_accumulator)
            .finish_non_exhaustive()
    }
}

/// Resizable panes.
pub fn splitter(orientation: Orientation) -> SplitterBuilder {
    SplitterBuilder {
        orientation,
        panes: Vec::new(),
        sizes_bind: None,
        settle_bind: None,
        proportional: false,
        snap_points: Vec::new(),
        snap_threshold: 8.0,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl SplitterBuilder {
    /// Sets the child constrained.
    pub fn child_constrained(
        mut self,
        node: impl IntoNode + 'static,
        initial: f32,
        min: f32,
        max: f32,
    ) -> Self {
        let mut entry = PaneEntry::new(node, initial);
        entry.min = min.max(0.0);
        entry.max = max.max(0.0);
        self.panes.push(entry);
        self
    }

    /// Adds a child.
    pub fn child(mut self, node: impl IntoNode + 'static, initial_size: f32) -> Self {
        self.panes.push(PaneEntry::new(node, initial_size));
        self
    }

    /// Adds a pane and returns its builder.
    pub fn pane(self, node: impl IntoNode + 'static) -> PaneBuilder {
        PaneBuilder::new(self, PaneEntry::new(node, 0.0))
    }

    /// Sets the min size.
    pub fn min_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.min = px.max(0.0);
        }
        self
    }

    /// Sets the max size.
    pub fn max_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.max = px.max(0.0);
        }
        self
    }

    /// Sets the sizes bind.
    pub const fn sizes_bind(mut self, signal: crate::reactive::Signal<Vec<f32>>) -> Self {
        self.sizes_bind = Some(signal);
        self
    }

    /// Sets the signal that gets the pane sizes when a drag or a key move ends.
    pub const fn settle_bind(mut self, signal: crate::reactive::Signal<Vec<f32>>) -> Self {
        self.settle_bind = Some(signal);
        self
    }

    /// Keeps each pane's share when the splitter resizes; initial sizes are then shares.
    pub const fn proportional(mut self) -> Self {
        self.proportional = true;
        self
    }

    /// Sets the snap points.
    pub fn snap_points(mut self, handle_idx: usize, points: &[f32]) -> Self {
        self.snap_points.push((handle_idx, points.to_vec()));
        self
    }

    /// Sets the snap threshold.
    pub const fn snap_threshold(mut self, threshold: f32) -> Self {
        self.snap_threshold = threshold.max(0.0);
        self
    }

    fn root(mut self) -> NodeBuilder {
        let orientation = self.orientation;
        let count = self.panes.len();
        if self.proportional {
            self.normalize_proportional();
        }
        let root_layout = self.root_layout();
        let grows: Vec<f32> = self
            .panes
            .iter()
            .map(|p| shares_free_space(p.initial_size, p.grow))
            .collect();
        let limits = PaneLimits {
            sizes: self.panes.iter().map(|p| p.initial_size).collect(),
            mins: self.panes.iter().map(|p| p.min).collect(),
            maxs: self.panes.iter().map(|p| p.max).collect(),
            grows: grows.clone(),
            proportional: self.proportional,
        };
        let state = SplitterState::new(
            orientation,
            limits,
            self.snap_points,
            self.snap_threshold,
            Binds {
                sizes: self.sizes_bind,
                settle: self.settle_bind,
            },
        );
        let mut root = container().role(Role::GenericContainer).layout(root_layout);
        if self.proportional {
            root = fit_to_bounds(root, state.clone(), orientation, count);
        }

        for (pane_idx, (entry, grow)) in self.panes.into_iter().zip(grows).enumerate() {
            let pane = caller_style(
                build_pane(entry.content, pane_idx, state.clone(), grow),
                entry.style,
            );
            root = root.child(entry.layout.finish(pane));
            if pane_idx + 1 < count {
                root = root.child(build_handle(pane_idx, orientation, state.clone()));
            }
        }

        caller_style(root, self.style_accumulator)
    }

    fn normalize_proportional(&mut self) {
        for pane in &mut self.panes {
            pane.initial_size = if pane.initial_size > 0.0 {
                pane.initial_size
            } else {
                1.0
            };
            pane.grow = 0.0;
        }
    }

    fn root_layout(&self) -> LayoutStyle {
        let root_layout = match self.orientation {
            Orientation::Horizontal => LayoutStyle::flex_row().align_items(AlignItems::Stretch),
            Orientation::Vertical => LayoutStyle::flex_column().align_items(AlignItems::Stretch),
        };
        if self.proportional {
            root_layout.fill().min_size(0.0, 0.0)
        } else {
            root_layout
        }
    }
}

builder_impls!([] SplitterBuilder: paint(style_accumulator), widget, node);

impl From<SplitterBuilder> for NodeBuilder {
    fn from(builder: SplitterBuilder) -> Self {
        builder.into_node_builder()
    }
}

fn fit_to_bounds(
    root: NodeBuilder,
    state: SplitterState,
    orientation: Orientation,
    count: usize,
) -> NodeBuilder {
    let bounds = crate::reactive::signal((0.0, 0.0, 0.0, 0.0));
    #[allow(
        clippy::cast_precision_loss,
        reason = "a splitter has a handful of panes"
    )]
    let handles = HANDLE_PX * count.saturating_sub(1) as f32;
    crate::reactive::effect(move || {
        let (_, _, width, height) = bounds.get();
        let along = match orientation {
            Orientation::Horizontal => width,
            Orientation::Vertical => height,
        };
        state.fit(along - handles);
    });
    root.bounds_reactive(bounds)
}

fn build_handle(handle_idx: usize, orientation: Orientation, state: SplitterState) -> NodeBuilder {
    let node = container()
        .role(Role::Splitter)
        .hittable(true)
        .layout(handle_layout(orientation))
        .child(rule(orientation));
    let node = node
        .hover(|s| s.background(Token::SurfaceHover))
        .pressed(|s| s.background(Token::SurfacePressed));
    install_handle_keys(
        install_handle_hooks(node, state.clone(), handle_idx),
        state,
        handle_idx,
    )
}

fn handle_layout(orientation: Orientation) -> LayoutStyle {
    let style = match orientation {
        Orientation::Horizontal => LayoutStyle::flex_row().width(HANDLE_PX),
        Orientation::Vertical => LayoutStyle::flex_column().height(HANDLE_PX),
    };
    style
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
        .justify_content(JustifyContent::Center)
}

fn rule(orientation: Orientation) -> NodeBuilder {
    let thickness = f32::from(BORDER_WIDTH);
    let layout = match orientation {
        Orientation::Horizontal => LayoutStyle::block().width(thickness).fill_height(),
        Orientation::Vertical => LayoutStyle::block().height(thickness).fill_width(),
    };
    container()
        .hittable(false)
        .layout(layout.shrink(0.0))
        .class(RULE)
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
