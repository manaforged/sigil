use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::reactive::{Reactive, Signal};
use crate::style::{StateStyle, StyleOverlay, StyleVariant};
use crate::widgets::Widget;
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{
    Activate, DIMMED, Press, RELEASE_ENTER_SPACE, disabled_a11y, press, reactive_style_state,
};

static NODE_VARIANT: StyleVariant =
    StyleVariant::new(&[], StateStyle::new(&[], &[], &[], &[], DIMMED));

#[derive(Default)]
pub(crate) struct Wiring {
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
}

impl Wiring {
    const fn is_inert(&self) -> bool {
        self.disabled.is_none() && self.activate.is_none()
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
}

pub(crate) fn wire(mut node: NodeBuilder, overlay: StyleOverlay) -> NodeBuilder {
    let wiring = std::mem::take(node.wiring_mut());
    if overlay.is_empty() && wiring.is_inert() {
        return node;
    }
    let Wiring { disabled, activate } = wiring;
    let interaction = InteractionState::new(true).disabled(disabled);
    let node = disabled_a11y(node, disabled).stateful_style(NODE_VARIANT, overlay, move || {
        reactive_style_state(interaction, disabled)
    });
    match activate {
        Some(activate) => interaction.install(press(
            node,
            interaction,
            disabled,
            Press::Activate(activate),
            RELEASE_ENTER_SPACE,
        )),
        None if node.is_focusable() => {
            interaction.observe_style(node.focus_ring(interaction.focus_ring()))
        }
        None => interaction.observe_style(node),
    }
}
