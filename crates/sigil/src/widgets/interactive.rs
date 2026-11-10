use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::reactive::{Reactive, Signal};
use crate::style::{StateStyle, StyleOverlay, StyleVariant};
use crate::widgets::Widget;
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{
    Activate, DIMMED, RELEASE_ENTER_SPACE, disabled_a11y, press, reactive_style_state,
};

static NODE_VARIANT: StyleVariant =
    StyleVariant::new(&[], StateStyle::new(&[], &[], &[], &[], DIMMED));

#[derive(Default)]
pub(crate) struct Wiring {
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    state: Option<InteractionState>,
    variant: Option<StyleVariant>,
}

impl Wiring {
    const fn is_inert(&self) -> bool {
        self.disabled.is_none()
            && self.activate.is_none()
            && self.state.is_none()
            && self.variant.is_none()
    }
}

impl NodeBuilder {
    /// Disables the node: it dims, ignores pointer and keys, and reads as disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.wiring_mut().disabled = Some(disabled.into_signal());
        self
    }

    /// Runs `activate` on a primary click, Enter, or Space, and makes the node focusable.
    #[must_use]
    pub fn activate(self, activate: impl Fn() + 'static) -> Self {
        self.activate_with(Rc::new(activate))
    }

    pub(crate) fn activate_with(mut self, activate: Activate) -> Self {
        self.wiring_mut().activate = Some(activate);
        self
    }

    pub(crate) const fn disabled_by(mut self, disabled: Option<Signal<bool>>) -> Self {
        self.wiring_mut().disabled = disabled;
        self
    }

    pub(crate) const fn variant(mut self, variant: StyleVariant) -> Self {
        self.wiring_mut().variant = Some(variant);
        self
    }

    pub(crate) fn interaction(&mut self) -> InteractionState {
        let wiring = self.wiring_mut();
        let disabled = wiring.disabled;
        *wiring
            .state
            .get_or_insert_with(|| InteractionState::new(true).disabled(disabled))
    }
}

pub(crate) fn wire(mut node: NodeBuilder, overlay: StyleOverlay) -> NodeBuilder {
    let wiring = std::mem::take(node.wiring_mut());
    if overlay.is_empty() && wiring.is_inert() {
        return node;
    }
    let Wiring {
        disabled,
        activate,
        state,
        variant,
    } = wiring;
    let interaction = state
        .unwrap_or_else(|| InteractionState::new(true))
        .disabled(disabled);
    let node = disabled_a11y(node, disabled).stateful_style(
        variant.unwrap_or(NODE_VARIANT),
        overlay,
        move || reactive_style_state(interaction, disabled),
    );
    let node = if variant.is_some() {
        node.seal_style()
    } else {
        node
    };
    match activate {
        Some(activate) => interaction.install(press(
            node,
            interaction,
            disabled,
            &activate,
            RELEASE_ENTER_SPACE,
        )),
        None if node.is_focusable() => {
            interaction.observe_style(node.focus_ring(interaction.focus_ring()))
        }
        None => interaction.observe_style(node),
    }
}
