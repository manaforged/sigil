use smallvec::SmallVec;

use crate::kernel::{ImageBuilder, NodeBuilder, TextBuilder};
use crate::style::{StateSlot, StyleAtom};
use crate::widgets::{Edit, Sealed, StateTarget, Widget};

impl Sealed for TextBuilder {
    fn edit(mut self, edit: Edit) -> Self {
        self.node = edit.apply(self.node);
        self
    }

    fn map_target(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.map_node(f)
    }
}

impl StateTarget for TextBuilder {
    fn set_state(mut self, slot: StateSlot, atoms: SmallVec<[StyleAtom; 16]>) -> Self {
        self.node = self.node.set_state(slot, atoms);
        self
    }
}

impl Widget for TextBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.hooks
            .push(Box::new(move |node| node.map_keeping_kind(f)));
        self
    }
}

impl Sealed for ImageBuilder {
    fn edit(mut self, edit: Edit) -> Self {
        self.node = edit.apply(self.node);
        self
    }

    fn map_target(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.map_node(f)
    }
}

impl Widget for ImageBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.hooks
            .push(Box::new(move |node| node.map_keeping_kind(f)));
        self
    }
}
