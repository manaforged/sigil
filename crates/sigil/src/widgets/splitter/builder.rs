use super::super::interactive::interactive;
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::style::{BORDER_WIDTH, Paint, PaintTarget, Style, StyleAtom, StyleSlot, Token};

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::caller_style;
use super::drag::{Binds, PaneLimits, SplitterState, install_handle_hooks};
use super::keys::install_handle_keys;
use super::pane::{build_pane, shares_free_space};
use crate::style::StateVariants;
use crate::widgets::orientation::Orientation;
use crate::widgets::{Edit, Sealed, Widget};

pub(in crate::widgets) const HANDLE_PX: f32 = 6.0;

static RULE: &[StyleAtom] = &[Token::Border.bg()];

struct PaneEntry {
    content: Box<dyn IntoNode>,
    initial_size: f32,
    min: f32,
    max: f32,
    grow: f32,
}

/// Builder of a splitter.
pub struct SplitterBuilder {
    orientation: Orientation,
    panes: Vec<PaneEntry>,
    sizes_bind: Option<crate::reactive::Signal<Vec<f32>>>,
    settle_bind: Option<crate::reactive::Signal<Vec<f32>>>,
    proportional: bool,
    snap_points: Vec<(usize, Vec<f32>)>,
    snap_threshold: f32,
    layout_overrides: LayoutOverrides,
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
#[must_use]
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
    #[must_use]
    pub fn child_constrained(
        self,
        node: impl IntoNode + 'static,
        initial: f32,
        min: f32,
        max: f32,
    ) -> Self {
        self.pane(node).initial(initial).min(min).max(max).commit()
    }

    /// Adds a child.
    #[must_use]
    pub fn child(self, node: impl IntoNode + 'static, initial_size: f32) -> Self {
        self.pane(node).initial(initial_size).commit()
    }

    /// Adds a pane and returns its builder.
    pub fn pane(self, node: impl IntoNode + 'static) -> PaneBuilder {
        PaneBuilder {
            splitter: self,
            entry: PaneEntry {
                content: Box::new(node),
                initial_size: 0.0,
                min: 0.0,
                max: f32::INFINITY,
                grow: 0.0,
            },
        }
    }

    /// Sets the min size.
    #[must_use]
    pub fn min_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.min = px.max(0.0);
        }
        self
    }

    /// Sets the max size.
    #[must_use]
    pub fn max_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.max = px.max(0.0);
        }
        self
    }

    /// Sets the sizes bind.
    #[must_use]
    pub const fn sizes_bind(mut self, signal: crate::reactive::Signal<Vec<f32>>) -> Self {
        self.sizes_bind = Some(signal);
        self
    }

    /// Sets the signal that gets the pane sizes when a drag or a key move ends.
    #[must_use]
    pub const fn settle_bind(mut self, signal: crate::reactive::Signal<Vec<f32>>) -> Self {
        self.settle_bind = Some(signal);
        self
    }

    /// Keeps each pane's share when the splitter resizes; initial sizes are then shares.
    #[must_use]
    pub const fn proportional(mut self) -> Self {
        self.proportional = true;
        self
    }

    /// Sets the snap points.
    #[must_use]
    pub fn snap_points(mut self, handle_idx: usize, points: &[f32]) -> Self {
        self.snap_points.push((handle_idx, points.to_vec()));
        self
    }

    /// Sets the snap threshold.
    #[must_use]
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
            root = fit_to_bounds(root, state, orientation, count);
        }

        for (pane_idx, (entry, grow)) in self.panes.into_iter().zip(grows).enumerate() {
            root = root.child(build_pane(entry.content, pane_idx, state, grow));
            if pane_idx + 1 < count {
                root = root.child(build_handle(pane_idx, orientation, state));
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

/// Configures one pane of a `splitter` before it is committed.
#[must_use = "PaneBuilder must be committed by calling .commit(), .pane(...), or by passing it to cx.add()"]
pub struct PaneBuilder {
    splitter: SplitterBuilder,
    entry: PaneEntry,
}

impl core::fmt::Debug for PaneBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PaneBuilder")
            .field("splitter", &self.splitter)
            .finish_non_exhaustive()
    }
}

impl PaneBuilder {
    /// Sets the initial.
    pub const fn initial(mut self, px: f32) -> Self {
        self.entry.initial_size = px.max(0.0);
        self
    }

    /// Sets the min.
    pub const fn min(mut self, px: f32) -> Self {
        self.entry.min = px.max(0.0);
        self
    }

    /// Sets the max.
    pub const fn max(mut self, px: f32) -> Self {
        self.entry.max = px.max(0.0);
        self
    }

    /// Sets the grow.
    pub const fn grow(mut self, factor: f32) -> Self {
        self.entry.grow = factor.max(0.0);
        self
    }

    /// Adds a pane.
    pub fn pane(self, node: impl IntoNode + 'static) -> Self {
        self.commit().pane(node)
    }

    /// Returns the commit.
    #[must_use]
    pub fn commit(mut self) -> SplitterBuilder {
        self.splitter.panes.push(self.entry);
        self.splitter
    }
}

impl Sealed for PaneBuilder {
    fn edit(mut self, edit: Edit) -> Self {
        match edit {
            Edit::Layout(edit) => self.splitter.layout_overrides.edit(edit),
            other => self
                .splitter
                .layout_overrides
                .node_hook(move |node| other.apply(node)),
        }
        self
    }

    fn map_target(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.splitter.layout_overrides.node_hook(f);
        self
    }
}

impl Widget for PaneBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.splitter.layout_overrides.node_hook(f);
        self
    }
}

impl IntoNode for PaneBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.commit().build(cx)
    }
}

impl From<PaneBuilder> for NodeBuilder {
    fn from(builder: PaneBuilder) -> Self {
        builder.commit().into()
    }
}

impl PaintTarget for PaneBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.splitter.style_accumulator)
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
    let node = interactive(node)
        .hover(|s| s.bg(Token::SurfaceHover))
        .pressed(|s| s.bg(Token::SurfacePressed))
        .into_node_builder();
    install_handle_keys(
        install_handle_hooks(node, state, handle_idx),
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
