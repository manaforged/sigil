#![allow(clippy::redundant_pub_crate)]

use crate::events::{Key, LogicalKey, PhysicalKey};
use crate::hal::events::{for_each_named_key, for_each_physical_key};
use winit::keyboard::{
    Key as WinitKey, KeyCode as WinitKeyCode, NamedKey as WinitNamedKey,
    PhysicalKey as WinitPhysicalKey,
};

macro_rules! same_name_match {
    ($input:expr, $src:path => $dst:path, default: $default:expr, [$($v:ident),* $(,)?]) => {
        match $input {
            $( <$src>::$v => <$dst>::$v, )*
            _ => $default,
        }
    };
}

#[cfg(feature = "menu")]
pub(crate) use same_name_match;

pub(crate) fn map_logical_key(key: WinitKey) -> LogicalKey {
    match key {
        WinitKey::Named(WinitNamedKey::Space) => LogicalKey::character(" "),
        WinitKey::Named(WinitNamedKey::Hyper | WinitNamedKey::Super) => {
            LogicalKey::Named(Key::Meta)
        }
        WinitKey::Named(named) => LogicalKey::Named(map_named_key(named)),
        WinitKey::Character(value) => LogicalKey::character(value.as_str()),
        WinitKey::Dead(Some(ch)) => {
            let mut buffer = [0_u8; 4];
            LogicalKey::character(ch.encode_utf8(&mut buffer))
        }
        WinitKey::Dead(None) | WinitKey::Unidentified(_) => LogicalKey::Named(Key::Unidentified),
    }
}

pub(crate) const fn map_physical_key(key: WinitPhysicalKey) -> PhysicalKey {
    match key {
        WinitPhysicalKey::Code(WinitKeyCode::SuperLeft) => PhysicalKey::MetaLeft,
        WinitPhysicalKey::Code(WinitKeyCode::SuperRight) => PhysicalKey::MetaRight,
        WinitPhysicalKey::Code(code) => map_key_code(code),
        WinitPhysicalKey::Unidentified(_) => PhysicalKey::Unidentified,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one sequential pass; splitting would scatter the ordering it documents"
)]
const fn map_named_key(key: WinitNamedKey) -> Key {
    for_each_named_key!(same_name_match!(
        key,
        WinitNamedKey => Key,
        default: Key::Unidentified,
    ))
}

const fn map_key_code(code: WinitKeyCode) -> PhysicalKey {
    for_each_physical_key!(same_name_match!(
        code,
        WinitKeyCode => PhysicalKey,
        default: PhysicalKey::Unidentified,
    ))
}

#[cfg(test)]
#[path = "key_map_tests.rs"]
mod tests;
