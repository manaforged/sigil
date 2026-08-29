use crate::StyleAtom;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "intentional, reviewed trade-off"
)]
pub struct StyleState {
    pub disabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StateSlot {
    #[default]
    Rest,
    Focused,
    Hovered,
    Pressed,
    Disabled,
}

impl StyleState {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateStyle {
    rest: &'static [StyleAtom],
    focused: &'static [StyleAtom],
    hovered: &'static [StyleAtom],
    pressed: &'static [StyleAtom],
    disabled: &'static [StyleAtom],
}

impl StateStyle {
    pub const EMPTY: Self = Self {
        rest: &[],
        focused: &[],
        hovered: &[],
        pressed: &[],
        disabled: &[],
    };

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

    #[must_use]
    pub const fn with_rest(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.rest = atoms;
        self
    }

    #[must_use]
    pub const fn with_focused(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.focused = atoms;
        self
    }

    #[must_use]
    pub const fn with_hovered(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.hovered = atoms;
        self
    }

    #[must_use]
    pub const fn with_pressed(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.pressed = atoms;
        self
    }

    #[must_use]
    pub const fn with_disabled(mut self, atoms: &'static [StyleAtom]) -> Self {
        self.disabled = atoms;
        self
    }

    #[must_use]
    pub const fn rest(self) -> &'static [StyleAtom] {
        self.rest
    }

    #[must_use]
    pub const fn focused(self) -> &'static [StyleAtom] {
        self.focused
    }

    #[must_use]
    pub const fn hovered(self) -> &'static [StyleAtom] {
        self.hovered
    }

    #[must_use]
    pub const fn pressed(self) -> &'static [StyleAtom] {
        self.pressed
    }

    #[must_use]
    pub const fn disabled(self) -> &'static [StyleAtom] {
        self.disabled
    }

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyleVariant {
    base: &'static [StyleAtom],
    states: StateStyle,
}

impl StyleVariant {
    #[must_use]
    pub const fn new(base: &'static [StyleAtom], states: StateStyle) -> Self {
        Self { base, states }
    }

    #[must_use]
    pub const fn base(self) -> &'static [StyleAtom] {
        self.base
    }

    #[must_use]
    pub const fn state(self, state: StyleState) -> &'static [StyleAtom] {
        self.states.resolve(state)
    }

    #[must_use]
    pub const fn states(self) -> StateStyle {
        self.states
    }
}
