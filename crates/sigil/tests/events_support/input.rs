#![allow(dead_code, unused_imports)]

use sigil::events::Event;
use sigil::events::{LogicalKey, *};

pub(crate) fn ctrl_s() -> Event {
    Event::key_down(LogicalKey::character("s"))
        .physical(PhysicalKey::KeyS)
        .modifiers(ModifiersState::CONTROL)
}
