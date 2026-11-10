use crate::kernel::NodeBuilder;
use crate::kernel::hooks::NodeHooks;
use crate::layout::LayoutStyle;
use crate::widgets::LayoutEdit;

#[derive(Clone, Debug, Default)]
pub(crate) struct LayoutOverrides {
    edits: Vec<LayoutEdit>,
    hooks: NodeHooks,
}

impl LayoutOverrides {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            edits: Vec::new(),
            hooks: NodeHooks::new(),
        }
    }

    pub(crate) fn edit(&mut self, edit: LayoutEdit) {
        self.edits.push(edit);
    }

    pub(crate) fn node_hook(&mut self, hook: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) {
        self.hooks.push(Box::new(hook));
    }

    #[must_use]
    pub(crate) fn finish(self, node: NodeBuilder) -> NodeBuilder {
        let node = if self.edits.is_empty() {
            node
        } else {
            let edits = self.edits;
            node.mutate(move |node| edits.iter().for_each(|edit| edit.install(node)))
        };
        self.hooks.finish(node)
    }

    #[must_use]
    pub(crate) fn width_hint(&self) -> Option<f32> {
        self.apply(LayoutStyle::default()).0.size.width.px()
    }

    #[must_use]
    pub(crate) fn height_hint(&self) -> Option<f32> {
        self.apply(LayoutStyle::default()).0.size.height.px()
    }

    #[must_use]
    pub(crate) fn fills_height(&self) -> bool {
        self.apply(LayoutStyle::default())
            .data()
            .size
            .height
            .fills()
    }

    #[must_use]
    pub(crate) fn fills_width(&self) -> bool {
        self.apply(LayoutStyle::default()).data().size.width.fills()
    }

    #[must_use]
    pub(crate) fn apply(&self, base: LayoutStyle) -> LayoutStyle {
        let mut style = base;
        for edit in &self.edits {
            edit.apply(&mut style);
        }
        style
    }
}

#[cfg(test)]
#[path = "layout_ext_tests.rs"]
mod tests;
