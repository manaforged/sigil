//! Deterministic state-style resolution primitives.

use crate::StyleAtom;

/// Widget interaction flags used by state-driven style resolution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct StyleState {
    /// The widget is disabled.
    pub disabled: bool,
    /// The pointer is hovering the widget.
    pub hovered: bool,
    /// The widget is being actively pressed or dragged.
    pub pressed: bool,
    /// The widget owns keyboard focus.
    pub focused: bool,
}

/// Resolved state slot. Priority is disabled, pressed, hovered, focused, rest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StateSlot {
    /// No interactive state applies.
    #[default]
    Rest,
    /// Keyboard focus applies.
    Focused,
    /// Pointer hover applies.
    Hovered,
    /// Pointer/key press applies.
    Pressed,
    /// Widget is disabled.
    Disabled,
}

impl StyleState {
    /// Return the single state slot used for style overlay resolution.
    #[must_use]
    pub const fn slot(self) -> StateSlot {
        if self.disabled {
            StateSlot::Disabled
        } else if self.pressed {
            StateSlot::Pressed
        } else if self.hovered {
            StateSlot::Hovered
        } else if self.focused {
            StateSlot::Focused
        } else {
            StateSlot::Rest
        }
    }
}

/// Atom overlays for a widget's interaction states.
///
/// Construct via [`StateStyle::EMPTY`] and add slots incrementally with
/// [`with_rest`](Self::with_rest) / [`with_focused`](Self::with_focused) /
/// [`with_hovered`](Self::with_hovered) / [`with_pressed`](Self::with_pressed) /
/// [`with_disabled`](Self::with_disabled), or build all five at once
/// with [`new`](Self::new). All constructors are `const` so widget
/// crates can define state-style tables as `static` values.
///
/// Slots are independently inspectable via the matching getters
/// ([`rest`](Self::rest), etc.) so tests don't need pointer equality on
/// the full struct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateStyle {
    rest: &'static [StyleAtom],
    focused: &'static [StyleAtom],
    hovered: &'static [StyleAtom],
    pressed: &'static [StyleAtom],
    disabled: &'static [StyleAtom],
}

impl StateStyle {
    /// All-empty state style. Start here and layer slots on with the
    /// `with_*` builders.
    pub const EMPTY: Self = Self {
        rest: &[],
        focused: &[],
        hovered: &[],
        pressed: &[],
        disabled: &[],
    };

    /// Build state overlays with every slot specified explicitly.
    #[must_use]
    pub const fn new(
        rest: &'static [StyleAtom],
        focused: &'static [StyleAtom],
        hovered: &'static [StyleAtom],
        pressed: &'static [StyleAtom],
        disabled: &'static [StyleAtom],
    ) -> Self {
        Self {
            rest,
            focused,
            hovered,
            pressed,
            disabled,
        }
    }

    /// Replace the rest-slot overlay.
    #[must_use]
    pub const fn with_rest(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.rest = atoms;
        self
    }

    /// Replace the focused-slot overlay.
    #[must_use]
    pub const fn with_focused(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.focused = atoms;
        self
    }

    /// Replace the hovered-slot overlay.
    #[must_use]
    pub const fn with_hovered(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.hovered = atoms;
        self
    }

    /// Replace the pressed-slot overlay.
    #[must_use]
    pub const fn with_pressed(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.pressed = atoms;
        self
    }

    /// Replace the disabled-slot overlay.
    #[must_use]
    pub const fn with_disabled(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.disabled = atoms;
        self
    }

    /// Rest-slot overlay.
    #[must_use]
    pub const fn rest(self) -> &'static [StyleAtom] {
        self.rest
    }

    /// Focused-slot overlay.
    #[must_use]
    pub const fn focused(self) -> &'static [StyleAtom] {
        self.focused
    }

    /// Hovered-slot overlay.
    #[must_use]
    pub const fn hovered(self) -> &'static [StyleAtom] {
        self.hovered
    }

    /// Pressed-slot overlay.
    #[must_use]
    pub const fn pressed(self) -> &'static [StyleAtom] {
        self.pressed
    }

    /// Disabled-slot overlay.
    #[must_use]
    pub const fn disabled(self) -> &'static [StyleAtom] {
        self.disabled
    }

    /// Return the overlay atoms for a state.
    #[must_use]
    pub const fn resolve(self, state: StyleState) -> &'static [StyleAtom] {
        match state.slot() {
            StateSlot::Rest => self.rest,
            StateSlot::Focused => self.focused,
            StateSlot::Hovered => self.hovered,
            StateSlot::Pressed => self.pressed,
            StateSlot::Disabled => self.disabled,
        }
    }
}

impl Default for StateStyle {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Variant base plus state overlays.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyleVariant {
    base: &'static [StyleAtom],
    states: StateStyle,
}

impl StyleVariant {
    /// Build a style variant.
    #[must_use]
    pub const fn new(base: &'static [StyleAtom], states: StateStyle) -> Self {
        Self { base, states }
    }

    /// Atoms applied before state overlays.
    #[must_use]
    pub const fn base(self) -> &'static [StyleAtom] {
        self.base
    }

    /// Atoms applied after variant base for the current state.
    #[must_use]
    pub const fn state(self, state: StyleState) -> &'static [StyleAtom] {
        self.states.resolve(state)
    }

    /// Borrow the underlying [`StateStyle`].
    #[must_use]
    pub const fn states(self) -> StateStyle {
        self.states
    }
}
