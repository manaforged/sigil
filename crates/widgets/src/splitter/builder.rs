use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, StyleAtom};

use super::atoms::{cached_height_atom, cached_width_atom, to_i16};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::drag::{SplitterState, install_handle_hooks};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterOrientation {
    Horizontal,
    Vertical,
}

const HANDLE_PX: f32 = 6.0;

struct PaneEntry {
    content: Box<dyn IntoNode>,
    initial_size: f32,
    min: f32,
    max: f32,
    grow: f32,
}

pub struct SplitterBuilder {
    orientation: SplitterOrientation,
    panes: Vec<PaneEntry>,
    sizes_bind: Option<sigil_reactive::Signal<Vec<f32>>>,
    snap_points: Vec<(usize, Vec<f32>)>,
    snap_threshold: f32,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub const fn splitter(orientation: SplitterOrientation) -> SplitterBuilder {
    SplitterBuilder {
        orientation,
        panes: Vec::new(),
        sizes_bind: None,
        snap_points: Vec::new(),
        snap_threshold: 8.0,
        layout_overrides: LayoutOverrides::new(),
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
    pub fn child(mut self, node: impl IntoNode + 'static, initial_size: f32) -> Self {
        self.panes.push(PaneEntry {
            content: Box::new(node),
            initial_size: initial_size.max(0.0),
            min: 0.0,
            max: f32::INFINITY,
            grow: 0.0,
        });
        self
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
    pub const fn sizes_bind(mut self, signal: sigil_reactive::Signal<Vec<f32>>) -> Self {
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

        let sizes: Vec<f32> = self.panes.iter().map(|p| p.initial_size).collect();
        let mins: Vec<f32> = self.panes.iter().map(|p| p.min).collect();
        let maxs: Vec<f32> = self.panes.iter().map(|p| p.max).collect();
        let grows: Vec<f32> = self.panes.iter().map(|p| p.grow).collect();

        let state = SplitterState::new(orientation, sizes, mins, maxs, grows);

        for (handle_idx, pts) in self.snap_points {
            state.set_snap(handle_idx, pts);
        }
        state.set_snap_threshold(self.snap_threshold);

        if let Some(signal) = self.sizes_bind {
            state.bind_sizes(signal);
        }

        let root_layout = match orientation {
            SplitterOrientation::Horizontal => {
                LayoutStyle::flex_row().align_items(AlignItems::Stretch)
            }
            SplitterOrientation::Vertical => {
                LayoutStyle::flex_column().align_items(AlignItems::Stretch)
            }
        };

        let mut root = container()
            .role(Role::GenericContainer)
            .layout(root_layout.clone());

        for (pane_idx, entry) in self.panes.into_iter().enumerate() {
            root = root.child(build_pane(
                entry.content,
                pane_idx,
                orientation,
                state,
                entry.grow,
            ));
            if pane_idx + 1 < count {
                root = root.child(build_handle(pane_idx, orientation, state));
            }
        }

        root.layout(self.layout_overrides.apply(root_layout))
    }
}

impl IntoNode for SplitterBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
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
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.commit().build(cx)
    }
}

impl From<PaneBuilder> for NodeBuilder {
    fn from(builder: PaneBuilder) -> Self {
        builder.commit().into()
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

fn build_pane(
    content: Box<dyn IntoNode>,
    pane_idx: usize,
    orientation: SplitterOrientation,
    state: SplitterState,
    grow: f32,
) -> NodeBuilder {
    let initial_px = state.size_at(pane_idx);
    let initial_layout = pane_layout(orientation, initial_px, grow);
    let node = container()
        .role(Role::GenericContainer)
        .layout(initial_layout);
    let node = if grow > 0.0 {
        node
    } else {
        node.style_reactive(move || pane_size_atom(orientation, state.size_at(pane_idx)))
    };
    node.child(content)
}

fn pane_layout(orientation: SplitterOrientation, size_px: f32, grow: f32) -> LayoutStyle {
    let style = match orientation {
        SplitterOrientation::Horizontal => {
            let style = LayoutStyle::flex_column();
            if grow > 0.0 {
                if size_px > 0.0 {
                    style.width(size_px)
                } else {
                    style
                }
            } else {
                style.width(size_px)
            }
        }
        SplitterOrientation::Vertical => {
            let style = LayoutStyle::flex_column();
            if grow > 0.0 {
                if size_px > 0.0 {
                    style.height(size_px)
                } else {
                    style
                }
            } else {
                style.height(size_px)
            }
        }
    };
    if grow > 0.0 { style.grow(grow) } else { style }
}

fn pane_size_atom(orientation: SplitterOrientation, size_px: f32) -> &'static [StyleAtom] {
    let px = to_i16(size_px);
    match orientation {
        SplitterOrientation::Horizontal => cached_width_atom(px),
        SplitterOrientation::Vertical => cached_height_atom(px),
    }
}

fn build_handle(
    handle_idx: usize,
    orientation: SplitterOrientation,
    state: SplitterState,
) -> NodeBuilder {
    let layout = handle_layout(orientation);
    let node = container()
        .role(Role::Splitter)
        .layout(layout)
        .class(&[StyleAtom::BackgroundThemeRef("border")] as &[_]);
    install_handle_hooks(node, state, handle_idx)
}

fn handle_layout(orientation: SplitterOrientation) -> LayoutStyle {
    match orientation {
        SplitterOrientation::Horizontal => LayoutStyle::flex_column()
            .width(HANDLE_PX)
            .align_items(AlignItems::Stretch),
        SplitterOrientation::Vertical => LayoutStyle::flex_column()
            .height(HANDLE_PX)
            .align_items(AlignItems::Stretch),
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
