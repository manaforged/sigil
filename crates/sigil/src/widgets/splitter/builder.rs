use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::{
    BORDER_WIDTH, Paint, PaintTarget, StateVariants, Style, StyleAtom, StyleSlot, Token,
};

use super::super::interactive::interactive;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::drag::{PaneLimits, SplitterState, install_handle_hooks};
use super::keys::install_handle_keys;
use super::pane::{build_pane, shares_free_space};
use crate::widgets::orientation::Orientation;

pub(in crate::widgets) const HANDLE_PX: f32 = 6.0;

static RULE: &[StyleAtom] = &[Token::Border.bg()];

struct PaneEntry {
    content: Box<dyn IntoNode>,
    initial_size: f32,
    min: f32,
    max: f32,
    grow: f32,
}

pub struct SplitterBuilder {
    orientation: Orientation,
    panes: Vec<PaneEntry>,
    sizes_bind: Option<crate::reactive::Signal<Vec<f32>>>,
    snap_points: Vec<(usize, Vec<f32>)>,
    snap_threshold: f32,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

#[must_use]
pub fn splitter(orientation: Orientation) -> SplitterBuilder {
    SplitterBuilder {
        orientation,
        panes: Vec::new(),
        sizes_bind: None,
        snap_points: Vec::new(),
        snap_threshold: 8.0,
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl SplitterBuilder {
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

    #[must_use]
    pub fn child(self, node: impl IntoNode + 'static, initial_size: f32) -> Self {
        self.pane(node).initial(initial_size).commit()
    }

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

    #[must_use]
    pub fn min_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.min = px.max(0.0);
        }
        self
    }

    #[must_use]
    pub fn max_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.max = px.max(0.0);
        }
        self
    }

    #[must_use]
    pub const fn sizes_bind(mut self, signal: crate::reactive::Signal<Vec<f32>>) -> Self {
        self.sizes_bind = Some(signal);
        self
    }

    #[must_use]
    pub fn snap_points(mut self, handle_idx: usize, points: &[f32]) -> Self {
        self.snap_points.push((handle_idx, points.to_vec()));
        self
    }

    #[must_use]
    pub const fn snap_threshold(mut self, threshold: f32) -> Self {
        self.snap_threshold = threshold.max(0.0);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let orientation = self.orientation;
        let count = self.panes.len();
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
        };
        let state = SplitterState::new(
            orientation,
            limits,
            self.snap_points,
            self.snap_threshold,
            self.sizes_bind,
        );
        let root_layout = match orientation {
            Orientation::Horizontal => LayoutStyle::flex_row().align_items(AlignItems::Stretch),
            Orientation::Vertical => LayoutStyle::flex_column().align_items(AlignItems::Stretch),
        };

        let mut root = container()
            .role(Role::GenericContainer)
            .layout(root_layout.clone());

        for (pane_idx, (entry, grow)) in self.panes.into_iter().zip(grows).enumerate() {
            root = root.child(build_pane(entry.content, pane_idx, state, grow));
            if pane_idx + 1 < count {
                root = root.child(build_handle(pane_idx, orientation, state));
            }
        }

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        let root = self.layout_overrides.finish(root);
        root.with(self.layout_overrides.apply(root_layout))
    }
}

impl IntoNode for SplitterBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

impl From<SplitterBuilder> for NodeBuilder {
    fn from(builder: SplitterBuilder) -> Self {
        builder.into_node_builder()
    }
}

#[must_use = "PaneBuilder must be committed by calling .commit(), .pane(...), or by passing it to cx.add()"]
pub struct PaneBuilder {
    splitter: SplitterBuilder,
    entry: PaneEntry,
}

impl PaneBuilder {
    pub const fn initial(mut self, px: f32) -> Self {
        self.entry.initial_size = px.max(0.0);
        self
    }

    pub const fn min(mut self, px: f32) -> Self {
        self.entry.min = px.max(0.0);
        self
    }

    pub const fn max(mut self, px: f32) -> Self {
        self.entry.max = px.max(0.0);
        self
    }

    pub const fn grow(mut self, factor: f32) -> Self {
        self.entry.grow = factor.max(0.0);
        self
    }

    pub fn pane(self, node: impl IntoNode + 'static) -> Self {
        self.commit().pane(node)
    }

    #[must_use]
    pub fn commit(mut self) -> SplitterBuilder {
        self.splitter.panes.push(self.entry);
        self.splitter
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

impl PaintTarget for SplitterBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl PaintTarget for PaneBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.splitter.style_accumulator)
    }
}

impl WidgetLayoutExt for SplitterBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl WidgetLayoutExt for PaneBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.splitter.layout_overrides
    }
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
    let thickness = f32::from(BORDER_WIDTH.0);
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
