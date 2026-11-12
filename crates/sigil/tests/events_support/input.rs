#![allow(dead_code, unused_imports)]

use sigil::events::Event;
use sigil::events::{Key, *};

pub(crate) fn ctrl_s() -> Event {
    Event::key_down(Key::character("s"))
        .physical(PhysicalKey::KeyS)
        .modifiers(Modifiers::CTRL)
}
