//! Controller-glyph rendering with hot-swap.
//!
//! `glyph(InputAction)` resolves to the correct platform glyph based on
//! the current input device — `A` on `Xbox`, `✕` on `PlayStation`, `B` on
//! Switch, `Enter` on keyboard. Hot-swap is driven by a reactive
//! [`Signal<InputDevice>`] that the platform layer flips when a new
//! device becomes active.
//!
//! This first pass renders glyphs as Unicode text. Per-platform PNG
//! atlases at `crates/sigil/assets/glyphs/{xbox,ps,switch}/` ship in a
//! follow-up — the mapping table is structured so the swap is a
//! single edit per `(action, device)` pair.
//!
//! ```text
//! use sigil::prelude::*;
//! use sigil::input_glyphs::{glyph, InputAction};
//!
//! // Inside a node tree:
//! container().child(glyph(InputAction::Confirm))
//! ```
//!
//! The renderer subscribes to the device signal through a `derived`
//! reactive text, so calling `current_input_device().set(...)` from
//! the platform layer re-renders every visible glyph without any
//! manual invalidation.

use std::cell::Cell;

use sigil_kernel::{NodeBuilder, TextContent, fragment, text};
use sigil_reactive::{Signal, derived, signal};
use sigil_style::NodeStyleExt as _;

/// Semantic action a glyph can describe.
///
/// Add variants as the input vocabulary grows. The mapping table in
/// [`glyph_id`] is exhaustive over `(action, device)` pairs and will
/// fail to compile until every new variant is covered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputAction {
    /// Primary confirm / accept.
    Confirm,
    /// Cancel / back.
    Cancel,
    /// Open menu / pause.
    Menu,
    /// D-pad up.
    Up,
    /// D-pad down.
    Down,
    /// D-pad left.
    Left,
    /// D-pad right.
    Right,
    /// Jump action.
    Jump,
    /// Attack / fire action.
    Attack,
    /// Interact / use action.
    Interact,
}

/// Physical input device family the glyphs are drawn for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputDevice {
    /// `Xbox` / `XInput` controller.
    Xbox,
    /// `PlayStation` / `DualShock` / `DualSense` controller.
    Playstation,
    /// Nintendo Switch Pro / Joy-Con.
    Switch,
    /// Keyboard (and mouse).
    Keyboard,
}

/// Resolve `(action, device)` to its display glyph.
///
/// Returns a short Unicode string — a single letter, a shape, or a
/// short key name. Suitable for inline text rendering until atlas
/// assets ship.
///
/// The match is exhaustive over both enums so adding a new variant
/// is a compile-time prompt to fill in its mapping rather than a
/// silent runtime fallback.
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
    /// Cached signal handle for the current device.
    ///
    /// Lazy-initialised on first call to [`current_input_device`] so
    /// the signal is parented to whatever owner is active at that
    /// point (typically the app's top-level `Runtime::scope`).
    static DEVICE_SIGNAL: Cell<Option<Signal<InputDevice>>> = const { Cell::new(None) };
}

/// Reactive handle to the active input device.
///
/// Returns the same `Signal<InputDevice>` on every call within a
/// single reactive runtime. The signal defaults to
/// [`InputDevice::Keyboard`]; the platform layer should call
/// `current_input_device().set(...)` when it detects a new device.
///
/// # Panics
///
/// Panics if called outside a reactive scope on the very first call
/// for this thread — see
/// [`Runtime::scope`](sigil_reactive::Runtime::scope).
#[must_use]
pub fn current_input_device() -> Signal<InputDevice> {
    if let Some(existing) = DEVICE_SIGNAL.with(Cell::get) {
        return existing;
    }
    let fresh = signal(InputDevice::Keyboard);
    DEVICE_SIGNAL.with(|cell| cell.set(Some(fresh)));
    fresh
}

/// Reset the cached device signal.
///
/// Tests that create more than one reactive scope must call this
/// between scopes so the next scope creates a fresh signal under
/// its own owner. Production apps have a single long-lived scope
/// and never call this.
#[doc(hidden)]
pub fn reset_for_tests() {
    DEVICE_SIGNAL.with(|cell| cell.set(None));
}

/// Build a glyph node bound to the current input device.
///
/// The returned [`NodeBuilder`] is a fragment containing one reactive
/// text node whose content tracks both `action` (fixed at call time)
/// and [`current_input_device`] (reactive). Updating the device
/// signal re-renders every glyph created with `glyph` without any
/// manual invalidation.
///
/// # Panics
///
/// Panics if called outside a reactive scope (the inner `derived`
/// must register against a current owner).
#[must_use]
pub fn glyph(action: InputAction) -> NodeBuilder {
    let device = current_input_device();
    let label = derived(move || glyph_id(action, device.get()).to_owned());
    fragment().child(text(TextContent::Reactive(label)).class("text-on-surface"))
}

/// Build a glyph node for a specific device, ignoring hot-swap.
///
/// Useful for in-game tutorials that explicitly want to show every
/// platform's glyph side-by-side, or for previewing glyph packs in
/// settings menus.
#[must_use]
pub fn glyph_for(action: InputAction, device: InputDevice) -> NodeBuilder {
    fragment().child(text(glyph_id(action, device)).class("text-on-surface"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "input_glyphs_tests.rs"]
mod tests;
