use crate::kernel::{ImageBuilder, TextBuilder};
use crate::widgets::{StatePaint, StateTarget};

builder_impls!([] TextBuilder: kernel);
builder_impls!([] ImageBuilder: kernel);

impl StateTarget for TextBuilder {
    fn set_state(mut self, state: StatePaint) -> Self {
        self.node = self.node.set_state(state);
        self
    }
}
