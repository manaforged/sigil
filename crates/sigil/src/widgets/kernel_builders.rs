use crate::kernel::{ImageBuilder, NodeBuilder, TextBuilder};
use crate::widgets::{NodeEdit, Sealed, StatePaint, StateTarget, Widget};

impl Sealed for TextBuilder {
    fn edit(mut self, NodeEdit(edit): NodeEdit) -> Self {
        self.node = edit.apply(self.node);
        self
    }

    fn map_target(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.map_node(f)
    }
}

impl StateTarget for TextBuilder {
    fn set_state(mut self, state: StatePaint) -> Self {
        self.node = self.node.set_state(state);
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
    fn edit(mut self, NodeEdit(edit): NodeEdit) -> Self {
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
