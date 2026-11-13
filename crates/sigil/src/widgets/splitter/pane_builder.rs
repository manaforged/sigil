use crate::kernel::{Cx, IntoNode, NodeBuilder};
use crate::layout::Orientation;
use crate::layout::{LayoutStyle, Length};
use crate::style::{Paint, StateStyleBuilder};
use crate::widgets::{LayoutEdit, Widget};

use super::builder::{PaneEntry, SplitterBuilder};

/// Configures one pane of a `splitter` before it is committed.
#[must_use = "PaneBuilder must be committed by calling .commit(), .pane(...), or by passing it to cx.add()"]
pub struct PaneBuilder {
    pub(super) splitter: SplitterBuilder,
    pub(super) entry: PaneEntry,
}

impl core::fmt::Debug for PaneBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PaneBuilder")
            .field("splitter", &self.splitter)
            .finish_non_exhaustive()
    }
}

impl PaneBuilder {
    pub(super) const fn new(splitter: SplitterBuilder, entry: PaneEntry) -> Self {
        Self { splitter, entry }
    }

    /// Adds a pane.
    pub fn pane(self, node: impl IntoNode + 'static) -> Self {
        self.commit().pane(node)
    }

    /// Adds this pane to the splitter and returns the splitter.
    pub fn commit(mut self) -> SplitterBuilder {
        self.splitter.panes.push(self.entry);
        self.splitter
    }

    fn edit_layout(&mut self, edit: LayoutEdit) {
        self.entry.edit_layout(edit, self.splitter.orientation);
    }
}

impl PaneEntry {
    fn edit_layout(&mut self, edit: LayoutEdit, orientation: Orientation) {
        let main = match (&edit, orientation) {
            (LayoutEdit::Width(size), Orientation::Horizontal)
            | (LayoutEdit::Height(size), Orientation::Vertical) => size.px(),
            _ => None,
        };
        if let Some(px) = main {
            self.initial_size = px.max(0.0);
            return;
        }
        match edit {
            LayoutEdit::Grow(factor) => self.grow = factor.max(0.0),
            LayoutEdit::Merge(style) => {
                self.limit(&style, orientation);
                self.layout.edit(LayoutEdit::Merge(style));
            }
            edit => self.layout.edit(edit),
        }
    }

    fn limit(&mut self, style: &LayoutStyle, orientation: Orientation) {
        let data = style.data();
        let (min, max): (Length, Length) = match orientation {
            Orientation::Horizontal => (data.min_size.width, data.max_size.width),
            Orientation::Vertical => (data.min_size.height, data.max_size.height),
        };
        if let Some(px) = min.px() {
            self.min = px.max(0.0);
        }
        if let Some(px) = max.px() {
            self.max = px.max(0.0);
        }
    }
}

impl Widget for PaneBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.entry.layout.node_hook(f);
        self
    }

    crate::widgets::widget_layout_methods!(@edit edit_layout);
}

crate::widgets::impl_layout!([] PaneBuilder, @edit edit_layout);

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

impl Paint for PaneBuilder {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        self.entry.style.paint_mut()
    }
}
