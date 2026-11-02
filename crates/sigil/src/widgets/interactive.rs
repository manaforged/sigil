use crate::kernel::NodeBuilder;
use crate::reactive::{Reactive, Signal};
use crate::style::{ClassSource, Style};
use crate::widgets::Widget;
use crate::widgets::interaction::InteractionState;
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::shared::state_surface;

/// Builder of an interactive node.
pub struct InteractiveBuilder {
    inner: NodeBuilder,
    style: Style,
    disabled: Option<Signal<bool>>,
    focusable: bool,
    layout_overrides: LayoutOverrides,
}

/// Make any node hoverable, pressable, and focusable.
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
    /// Sets the focusable.
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.focusable = yes;
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    fn root(self) -> NodeBuilder {
        let mut style = self.style;
        let overlay = style.take_state_overlay();
        let base_atoms = style.into_atoms();
        let interaction = InteractionState::new(true);
        let mut node = self.inner.focusable(self.focusable);
        if !base_atoms.is_empty() {
            node = node.class(ClassSource::Owned(Box::new(base_atoms)));
        }
        let mut node = state_surface(node, interaction, self.disabled, overlay);
        if self.focusable {
            node = node.focus_ring(interaction.focus_ring());
        }
        interaction.observe_style(node)
    }
}

builder_impls!([] InteractiveBuilder: paint(style), widget, states(style), node(pub));
