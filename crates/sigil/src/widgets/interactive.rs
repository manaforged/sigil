use crate::kernel::NodeBuilder;
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::Reactive;
use crate::reactive::Signal;
use crate::style::{ClassSource, NodeStyleExt, PaintTarget, Style, StyleSlot};
use crate::widgets::interaction::InteractionState;
use crate::widgets::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::widgets::shared::state_surface;

pub struct InteractiveBuilder {
    inner: NodeBuilder,
    style: Style,
    disabled: Option<Signal<bool>>,
    focusable: bool,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn interactive(inner: NodeBuilder) -> InteractiveBuilder {
    InteractiveBuilder {
        inner,
        style: Style::new(),
        disabled: None,
        focusable: false,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl InteractiveBuilder {
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.focusable = yes;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let mut style = self.style;
        let overlay = style.take_state_overlay();
        let base_atoms = style.into_atoms();
        let interaction = InteractionState::new(true);
        let hooks = self.layout_overrides.hooks();
        let mut node = self.inner.focusable(self.focusable);
        if self.layout_overrides != LayoutOverrides::new() {
            node = node.layout(self.layout_overrides.apply(LayoutStyle::default()));
        }
        if !base_atoms.is_empty() {
            node = node.class(ClassSource::Owned(Box::new(base_atoms)));
        }
        let node = state_surface(node, interaction, self.disabled, overlay);
        let node = if self.focusable {
            node.focus_ring(interaction.focus_ring())
        } else {
            node
        };
        hooks.finish(interaction.observe_style(node))
    }
}

impl PaintTarget for InteractiveBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style)
    }
}

impl crate::style::StateHost for InteractiveBuilder {}

impl WidgetLayoutExt for InteractiveBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl crate::kernel::IntoNode for InteractiveBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}
