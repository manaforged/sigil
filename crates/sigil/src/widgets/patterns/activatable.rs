use std::rc::Rc;

use crate::kernel::{NodeBuilder, Role};

use super::super::interaction::InteractionState;
use super::super::shared::{Activate, Press, RELEASE_ENTER_SPACE, press};

pub(super) fn activatable(node: NodeBuilder, role: Role, activate: &Activate) -> NodeBuilder {
    let interaction = InteractionState::new(true);
    let node = press(
        node.role(role),
        interaction,
        None,
        Press::Activate(Rc::clone(activate)),
        RELEASE_ENTER_SPACE,
    );
    interaction.install(node)
}
