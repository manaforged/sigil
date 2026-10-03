use std::cell::RefCell;
use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::layout::{IntoPx, LayoutStyle};

mod events;

type NodeHook = Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>;

#[derive(Clone, Default)]
pub(crate) struct NodeHooks(Option<Rc<RefCell<Vec<NodeHook>>>>);

impl NodeHooks {
    fn push(&mut self, hook: NodeHook) {
        self.0
            .get_or_insert_with(Rc::default)
            .borrow_mut()
            .push(hook);
    }

    #[must_use]
    pub(crate) fn finish(&self, node: NodeBuilder) -> NodeBuilder {
        let Some(store) = &self.0 else {
            return node;
        };
        let hooks = std::mem::take(&mut *store.borrow_mut());
        hooks.into_iter().fold(node, |built, hook| hook(built))
    }
}

#[derive(Clone, Default)]
pub struct LayoutOverrides {
    full: Option<LayoutStyle>,
    width: Option<f32>,
    height: Option<f32>,
    grow: Option<f32>,
    hooks: NodeHooks,
}

impl core::fmt::Debug for LayoutOverrides {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LayoutOverrides")
            .field("full", &self.full)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("grow", &self.grow)
            .finish_non_exhaustive()
    }
}

impl PartialEq for LayoutOverrides {
    fn eq(&self, other: &Self) -> bool {
        self.full == other.full
            && self.width == other.width
            && self.height == other.height
            && self.grow == other.grow
    }
}

impl LayoutOverrides {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            full: None,
            width: None,
            height: None,
            grow: None,
            hooks: NodeHooks(None),
        }
    }

    pub(crate) fn node_hook(&mut self, hook: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) {
        self.hooks.push(Box::new(hook));
    }

    #[must_use]
    pub(crate) fn finish(&self, node: NodeBuilder) -> NodeBuilder {
        self.hooks.finish(node)
    }

    #[must_use]
    pub(crate) fn hooks(&self) -> NodeHooks {
        self.hooks.clone()
    }

    pub fn layout(&mut self, style: LayoutStyle) {
        self.full = Some(style);
    }

    pub const fn width(&mut self, px: f32) {
        self.width = Some(px);
    }

    pub const fn height(&mut self, px: f32) {
        self.height = Some(px);
    }

    pub const fn grow(&mut self, factor: f32) {
        self.grow = Some(factor);
    }

    #[must_use]
    pub const fn width_hint(&self) -> Option<f32> {
        self.width
    }

    #[must_use]
    pub const fn height_hint(&self) -> Option<f32> {
        self.height
    }

    #[must_use]
    pub fn apply(self, base: LayoutStyle) -> LayoutStyle {
        let mut style = base;
        if let Some(full) = &self.full {
            style.merge(full);
        }
        if let Some(width) = self.width {
            style = style.width(width);
            if let crate::layout::Dimension::Px(floor) = style.0.min_size.width
                && floor > width
            {
                let height = style.0.min_size.height;
                style = style.min_size(width, height);
            }
        }
        if let Some(height) = self.height {
            style = style.height(height);
        }
        if let Some(grow) = self.grow {
            style = style.grow(grow);
        }
        style
    }
}

pub trait WidgetLayoutExt: Sized {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides;

    #[must_use]
    fn layout(mut self, style: LayoutStyle) -> Self {
        LayoutOverrides::layout(self.layout_overrides_mut(), style);
        self
    }

    #[must_use]
    fn width(mut self, width: impl IntoPx) -> Self {
        LayoutOverrides::width(self.layout_overrides_mut(), width.into_px());
        self
    }

    #[must_use]
    fn height(mut self, height: impl IntoPx) -> Self {
        LayoutOverrides::height(self.layout_overrides_mut(), height.into_px());
        self
    }

    #[must_use]
    fn grow(mut self, factor: f32) -> Self {
        LayoutOverrides::grow(self.layout_overrides_mut(), factor);
        self
    }

    #[must_use]
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.layout_overrides_mut().node_hook(f);
        self
    }
}

#[cfg(test)]
#[path = "layout_ext_tests.rs"]
mod tests;
