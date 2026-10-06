use std::rc::Rc;

use crate::events::{Key, LogicalKey, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role};

use super::super::interaction::InteractionState;
use super::super::shared::{Activate, activate_key_reactive, activate_pointer_reactive};

pub(super) fn activatable(node: NodeBuilder, role: Role, activate: &Activate) -> NodeBuilder {
    let interaction = InteractionState::new(true);
    let pointer = Rc::clone(activate);
    let enter = Rc::clone(activate);
    let space_key = Rc::clone(activate);
    let node = node
        .role(role)
        .pointer_up(move |event| activate_pointer_reactive(event, interaction, None, &pointer))
        .key(Key::Enter, move |event| {
            activate_key_reactive(event, interaction, None, &enter);
        })
        .key(LogicalKey::character(" "), move |event| {
            activate_key_reactive(event, interaction, None, &space_key);
        });
    interaction.install(node)
}
