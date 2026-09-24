use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::style::{ClassSource, NodeStyleExt, Style, StyleAccess};
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::state_surface;

pub struct InteractiveBuilder {
    inner: NodeBuilder,
    style: Style,
    disabled: Option<Signal<bool>>,
    focusable: bool,
}

#[must_use]
pub fn interactive(inner: NodeBuilder) -> InteractiveBuilder {
    InteractiveBuilder {
        inner,
        style: Style::new(),
        disabled: None,
        focusable: false,
    }
}

impl InteractiveBuilder {
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.focusable = yes;
        self
    }

    #[must_use]
    pub const fn disabled_signal(mut self, signal: Signal<bool>) -> Self {
        self.disabled = Some(signal);
        self
    }

    #[must_use]
    pub fn class(mut self, classes: &str) -> Self {
        self.style = core::mem::take(&mut self.style).class(classes);
        self
    }

    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let mut style = self.style;
        let overlay = style.take_state_overlay();
        let base_atoms = style.into_atoms();
        let interaction = InteractionState::new(true);
        let mut node = self.inner.focusable(self.focusable);
        if !base_atoms.is_empty() {
            node = node.class(ClassSource::Owned(Box::new(base_atoms)));
        }
        let node = state_surface(node, interaction, self.disabled, overlay);
        let node = if self.focusable {
            node.focus_ring(interaction.focus_ring())
        } else {
            node
        };
        interaction.observe_style(node)
    }
}

impl StyleAccess for InteractiveBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl crate::style::StateHost for InteractiveBuilder {}

impl crate::kernel::IntoNode for InteractiveBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}
