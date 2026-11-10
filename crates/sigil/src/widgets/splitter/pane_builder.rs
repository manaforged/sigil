use crate::kernel::{Cx, IntoNode, NodeBuilder};
use crate::layout::{LayoutStyle, Length};
use crate::style::{PaintTarget, StyleSlot};
use crate::widgets::orientation::Orientation;
use crate::widgets::{Edit, LayoutEdit, Sealed, Widget};

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

    /// Returns the commit.
    #[must_use]
    pub fn commit(mut self) -> SplitterBuilder {
        self.splitter.panes.push(self.entry);
        self.splitter
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

impl Sealed for PaneBuilder {
    fn edit(mut self, edit: Edit) -> Self {
        match edit {
            Edit::Layout(edit) => self.entry.edit_layout(edit, self.splitter.orientation),
            other => self.entry.layout.node_hook(move |node| other.apply(node)),
        }
        self
    }

    fn map_target(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.entry.layout.node_hook(f);
        self
    }
}

impl Widget for PaneBuilder {
    fn map_node(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.map_target(f)
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
        StyleSlot(&mut self.entry.style)
    }
}
