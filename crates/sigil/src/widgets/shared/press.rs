use std::rc::Rc;

use crate::events::{Key, LogicalKey, Shortcut};
use crate::kernel::NodeBuilder;
use crate::reactive::Signal;
use crate::widgets::Widget;

use super::{Activate, BoolChange, disabled_now};
use crate::widgets::interaction::InteractionState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Trigger {
    Release,
    Enter,
    Space,
}

pub(crate) const RELEASE: &[Trigger] = &[Trigger::Release];
pub(crate) const RELEASE_ENTER: &[Trigger] = &[Trigger::Release, Trigger::Enter];
pub(crate) const RELEASE_ENTER_SPACE: &[Trigger] =
    &[Trigger::Release, Trigger::Enter, Trigger::Space];

impl Trigger {
    fn shortcut(self) -> Option<Shortcut> {
        match self {
            Self::Release => None,
            Self::Enter => Some(Key::Enter.into()),
            Self::Space => Some(LogicalKey::character(" ").into()),
        }
    }
}

pub(crate) fn flip(value: Signal<bool>, on_change: BoolChange) -> Activate {
    Rc::new(move || {
        let next = !value.peek();
        value.set(next);
        on_change(next);
    })
}

pub(crate) fn press(
    node: NodeBuilder,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    activate: &Activate,
    triggers: &[Trigger],
) -> NodeBuilder {
    triggers.iter().fold(node, |node, trigger| {
        let activate = Rc::clone(activate);
        match trigger.shortcut() {
            None => node.pointer_up(move |event| {
                if interaction.release_pointer(event) && !disabled_now(disabled) {
                    activate();
                }
            }),
            Some(shortcut) => node.key(shortcut, move |event| {
                if interaction.activate_key(event) && !disabled_now(disabled) {
                    activate();
                }
            }),
        }
    })
}
