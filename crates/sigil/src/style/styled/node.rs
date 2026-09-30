use crate::kernel::NodeBuilder;

use super::{StateHost, StyleAccess};
use crate::style::{ClassSource, NodeStyleExt, StateStyle, Style, StyleVariant};
use crate::widgets::interaction::InteractionState;

pub struct NodeStyledBuilder {
    inner: NodeBuilder,
    style: Style,
}

impl NodeStyledBuilder {
    #[must_use]
    pub fn wrap(inner: NodeBuilder) -> Self {
        Self {
            inner,
            style: Style::new(),
        }
    }

    #[must_use]
    pub const fn style(&self) -> &Style {
        &self.style
    }
    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        attach_style(self.inner, self.style)
    }
}

impl StyleAccess for NodeStyledBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl StateHost for NodeStyledBuilder {}

pub trait Styleable: Sized {
    #[must_use]
    fn styled(self) -> NodeStyledBuilder;
}

impl Styleable for NodeBuilder {
    fn styled(self) -> NodeStyledBuilder {
        NodeStyledBuilder::wrap(self)
    }
}

impl crate::kernel::IntoNode for NodeStyledBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

pub(crate) fn attach_style(builder: NodeBuilder, style: Style) -> NodeBuilder {
    if style.is_empty() {
        return builder;
    }
    let mut style = style;
    let overlay = style.take_state_overlay();
    let builder = if overlay.is_empty() {
        builder
    } else {
        let interaction = InteractionState::new(true);
        let builder = builder.stateful_style(
            StyleVariant::new(&[], StateStyle::EMPTY),
            overlay,
            move || interaction.style_state(),
        );
        interaction.observe_style(builder)
    };
    let base_atoms = style.into_atoms();
    if base_atoms.is_empty() {
        return builder;
    }
    builder.class(ClassSource::Owned(Box::new(base_atoms)))
}
