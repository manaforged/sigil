use std::rc::Rc;

use crate::kernel::NodeBuilder;
use crate::reactive::{Reactive, Signal};
use crate::style::{StateStyle, StyleOverlay, StyleState, StyleVariant};
use crate::widgets::Widget;
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{
    Activate, DIMMED, RELEASE_ENTER_SPACE, disabled_a11y, or_disabled, press,
};

static NODE_VARIANT: StyleVariant =
    StyleVariant::new(&[], StateStyle::new(&[], &[], &[], &[], DIMMED));

#[derive(Default)]
pub(crate) struct Wiring {
    disabled: Option<Signal<bool>>,
    activate: Option<Activate>,
    state: Option<InteractionState>,
    variant: Option<StyleVariant>,
    busy: Option<Signal<bool>>,
    active: bool,
}

impl Wiring {
    pub(crate) const fn is_inert(&self) -> bool {
        self.disabled.is_none()
            && self.activate.is_none()
            && self.state.is_none()
            && self.variant.is_none()
            && self.busy.is_none()
            && !self.active
    }
}

impl NodeBuilder {
    /// Disables the node: it dims, its press, key, and action handlers stop running, and it reads as disabled.
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

    pub(crate) fn disabled_by(mut self, disabled: Option<Signal<bool>>) -> Self {
        self.wiring_mut().disabled = disabled;
        self
    }

    pub(crate) fn active(mut self) -> Self {
        self.wiring_mut().active = true;
        self
    }

    pub(crate) fn busy_by(mut self, busy: Option<Signal<bool>>) -> Self {
        self.wiring_mut().busy = busy;
        self
    }

    pub(crate) fn variant(mut self, variant: StyleVariant) -> Self {
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
    let wiring = node.take_wiring();
    if overlay.is_empty() && wiring.is_inert() {
        return node;
    }
    let Wiring {
        disabled,
        activate,
        state,
        variant,
        busy,
        active,
    } = wiring;
    let look = disabled;
    let disabled = or_disabled(disabled, busy);
    let interaction = state
        .unwrap_or_else(|| InteractionState::new(true))
        .disabled(disabled);
    let node = disabled_a11y(node, disabled).stateful_style(
        variant.unwrap_or(NODE_VARIANT),
        overlay,
        move || {
            let now = interaction.style_state();
            let live = !disabled.is_some_and(Signal::get);
            StyleState {
                disabled: look.is_some_and(Signal::get),
                hovered: now.hovered && live,
                pressed: now.pressed && live,
                ..now
            }
        },
    );
    let node = node.seal_style();
    match activate {
        Some(activate) => interaction.install(press(
            node,
            interaction,
            disabled,
            &activate,
            RELEASE_ENTER_SPACE,
        )),
        None if active => interaction.install(node),
        None if node.is_focusable() => {
            interaction.observe_style(node.focus_ring(interaction.focus_ring()))
        }
        None => interaction.observe_style(node),
    }
}
