//! Glyph helpers for key and input rendering.
use std::cell::Cell;

use crate::kernel::{NodeBuilder, TextContent, fragment, text};
use crate::reactive::{Signal, derived, signal};
use crate::style::NodeStyleExt as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputAction {
    Confirm,
    Cancel,
    Menu,
    Up,
    Down,
    Left,
    Right,
    Jump,
    Attack,
    Interact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputDevice {
    Xbox,
    Playstation,
    Switch,
    Keyboard,
}

#[must_use]
#[allow(
    clippy::match_same_arms,
    reason = "the (action, device) -> glyph mapping is a flat lookup table; \
              merging arms by output value would obscure which devices share a \
              physical button"
)]
pub const fn glyph_id(action: InputAction, device: InputDevice) -> &'static str {
    use InputAction as A;
    use InputDevice as D;
    match (action, device) {
        (A::Confirm, D::Xbox) => "A",
        (A::Confirm, D::Playstation) => "✕",
        (A::Confirm, D::Switch) => "B",
        (A::Confirm, D::Keyboard) => "Enter",
        (A::Cancel, D::Xbox) => "B",
        (A::Cancel, D::Playstation) => "○",
        (A::Cancel, D::Switch) => "A",
        (A::Cancel, D::Keyboard) => "Esc",
        (A::Menu, D::Xbox) => "☰",
        (A::Menu, D::Playstation) => "≡",
        (A::Menu, D::Switch) => "+",
        (A::Menu, D::Keyboard) => "Tab",
        (A::Up, D::Xbox | D::Playstation | D::Switch) => "↑",
        (A::Up, D::Keyboard) => "W",
        (A::Down, D::Xbox | D::Playstation | D::Switch) => "↓",
        (A::Down, D::Keyboard) => "S",
        (A::Left, D::Xbox | D::Playstation | D::Switch) => "←",
        (A::Left, D::Keyboard) => "A",
        (A::Right, D::Xbox | D::Playstation | D::Switch) => "→",
        (A::Right, D::Keyboard) => "D",
        (A::Jump, D::Xbox) => "A",
        (A::Jump, D::Playstation) => "✕",
        (A::Jump, D::Switch) => "B",
        (A::Jump, D::Keyboard) => "Space",
        (A::Attack, D::Xbox) => "X",
        (A::Attack, D::Playstation) => "□",
        (A::Attack, D::Switch) => "Y",
        (A::Attack, D::Keyboard) => "LMB",
        (A::Interact, D::Xbox) => "Y",
        (A::Interact, D::Playstation) => "△",
        (A::Interact, D::Switch) => "X",
        (A::Interact, D::Keyboard) => "E",
    }
}

thread_local! {
    static DEVICE_SIGNAL: Cell<Option<Signal<InputDevice>>> = const { Cell::new(None) };
}

#[must_use]
pub fn current_input_device() -> Signal<InputDevice> {
    if let Some(existing) = DEVICE_SIGNAL.with(Cell::get) {
        return existing;
    }
    let fresh = signal(InputDevice::Keyboard);
    DEVICE_SIGNAL.with(|cell| cell.set(Some(fresh)));
    fresh
}

#[doc(hidden)]
pub fn reset_for_tests() {
    DEVICE_SIGNAL.with(|cell| cell.set(None));
}

#[must_use]
pub fn glyph(action: InputAction) -> NodeBuilder {
    let device = current_input_device();
    let label = derived(move || glyph_id(action, device.get()).to_owned());
    fragment().child(text(TextContent::Reactive(label)).class("text-on-surface"))
}

#[must_use]
pub fn glyph_for(action: InputAction, device: InputDevice) -> NodeBuilder {
    fragment().child(text(glyph_id(action, device)).class("text-on-surface"))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "input_glyphs_tests.rs"]
mod tests;
