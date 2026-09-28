#![allow(dead_code, unused_imports)]

use sigil::events::prelude::*;
use sigil::hal::LogicalKey;
pub(crate) use sigil_kittest::input::{
    chord, ime_event, key_down, key_event, pointer_down, pointer_drag, pointer_move, pointer_up,
    scroll_lines, tab, type_char, window_blur,
};

pub(crate) fn ctrl_s() -> ShellEvent {
    chord(
        LogicalKey::character("s"),
        PhysicalKey::KeyS,
        ModifiersState::CONTROL,
    )
}
