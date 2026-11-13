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
}

impl Wiring {
    pub(crate) const fn is_inert(&self) -> bool {
        self.disabled.is_none()
            && self.activate.is_none()
            && self.state.is_none()
            && self.variant.is_none()
            && self.busy.is_none()
    }
}

impl NodeBuilder {
    /// Disables the node: it dims, its press, key, and action handlers stop running, and it reads as disabled.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.wiring_mut().disabled = Some(disabled.into_signal());
        self
    }

    /// Runs `activate` on a primary click, Enter, or Space, and makes the node focusable.
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

    /// Marks the node as busy while `busy` is true; it dims, stops handling input, and ignores activation.
    pub fn busy(mut self, busy: impl Reactive<bool>) -> Self {
        self.wiring_mut().busy = Some(busy.into_signal());
        self
    }

    /// Marks the node as invalid while `invalid` is true, combining prior calls via logical OR.
    pub fn invalid(self, invalid: impl Reactive<bool>) -> Self {
        let invalid = invalid.into_signal();
        self.mutate(move |node| {
            let combined = match node.a11y.invalid {
                Some(prior) => crate::reactive::memo(move || prior.get() || invalid.get()),
                None => invalid,
            };
            node.a11y.invalid = Some(combined);
        })
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
        return if node.is_focusable() {
            install_keyboard_focus_ring(node)
        } else {
            node
        };
    }
    let Wiring {
        disabled,
        activate,
        state,
        variant,
        busy,
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
        None if node.is_focusable() => interaction.observe_style(
            crate::style::builder_ext::default_focus_ring(node, interaction.focus_ring()),
        ),
        None => interaction.observe_style(node),
    }
}

fn install_keyboard_focus_ring(node: NodeBuilder) -> NodeBuilder {
    let ring = crate::reactive::signal(false);
    crate::style::builder_ext::default_focus_ring(node, ring)
        .focus_in(move || {
            let show = crate::events::focus_from_keyboard();
            if ring.peek() != show {
                ring.set(show);
            }
        })
        .focus_out(move || {
            if ring.peek() {
                ring.set(false);
            }
        })
}
