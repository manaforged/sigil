use crate::style::StyleAtom;

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PerState<T> {
    rest: T,
    focused: T,
    hovered: T,
    pressed: T,
    disabled: T,
}

impl<T> PerState<T> {
    pub(crate) const fn new(rest: T, focused: T, hovered: T, pressed: T, disabled: T) -> Self {
        Self {
            rest,
            focused,
            hovered,
            pressed,
            disabled,
        }
    }

    pub(crate) const fn get(&self, slot: StateSlot) -> &T {
        match slot {
            StateSlot::Rest => &self.rest,
            StateSlot::Focused => &self.focused,
            StateSlot::Hovered => &self.hovered,
            StateSlot::Pressed => &self.pressed,
            StateSlot::Disabled => &self.disabled,
        }
    }

    pub(crate) const fn at(&self, slot: StateSlot) -> T
    where
        T: Copy,
    {
        *self.get(slot)
    }

    pub(crate) const fn get_mut(&mut self, slot: StateSlot) -> &mut T {
        match slot {
            StateSlot::Rest => &mut self.rest,
            StateSlot::Focused => &mut self.focused,
            StateSlot::Hovered => &mut self.hovered,
            StateSlot::Pressed => &mut self.pressed,
            StateSlot::Disabled => &mut self.disabled,
        }
    }

    pub(crate) const fn each(&self) -> [&T; 5] {
        [
            &self.rest,
            &self.focused,
            &self.hovered,
            &self.pressed,
            &self.disabled,
        ]
    }

    pub(crate) const fn each_mut(&mut self) -> [&mut T; 5] {
        [
            &mut self.rest,
            &mut self.focused,
            &mut self.hovered,
            &mut self.pressed,
            &mut self.disabled,
        ]
    }
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
    slots: PerState<&'static [StyleAtom]>,
}

impl StateStyle {
    pub const EMPTY: Self = Self {
        slots: PerState::new(&[], &[], &[], &[], &[]),
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
            slots: PerState::new(rest, focused, hovered, pressed, disabled),
        }
    }

    #[must_use]
    pub const fn with(mut self, slot: StateSlot, atoms: &'static [StyleAtom]) -> Self {
        *self.slots.get_mut(slot) = atoms;
        self
    }

    #[must_use]
    pub const fn get(self, slot: StateSlot) -> &'static [StyleAtom] {
        self.slots.at(slot)
    }

    #[must_use]
    pub const fn with_rest(self, atoms: &'static [StyleAtom]) -> Self {
        self.with(StateSlot::Rest, atoms)
    }

    #[must_use]
    pub const fn with_focused(self, atoms: &'static [StyleAtom]) -> Self {
        self.with(StateSlot::Focused, atoms)
    }

    #[must_use]
    pub const fn with_hovered(self, atoms: &'static [StyleAtom]) -> Self {
        self.with(StateSlot::Hovered, atoms)
    }

    #[must_use]
    pub const fn with_pressed(self, atoms: &'static [StyleAtom]) -> Self {
        self.with(StateSlot::Pressed, atoms)
    }

    #[must_use]
    pub const fn with_disabled(self, atoms: &'static [StyleAtom]) -> Self {
        self.with(StateSlot::Disabled, atoms)
    }

    #[must_use]
    pub const fn rest(self) -> &'static [StyleAtom] {
        self.get(StateSlot::Rest)
    }

    #[must_use]
    pub const fn focused(self) -> &'static [StyleAtom] {
        self.get(StateSlot::Focused)
    }

    #[must_use]
    pub const fn hovered(self) -> &'static [StyleAtom] {
        self.get(StateSlot::Hovered)
    }

    #[must_use]
    pub const fn pressed(self) -> &'static [StyleAtom] {
        self.get(StateSlot::Pressed)
    }

    #[must_use]
    pub const fn disabled(self) -> &'static [StyleAtom] {
        self.get(StateSlot::Disabled)
    }

    #[must_use]
    pub const fn resolve(self, state: StyleState) -> &'static [StyleAtom] {
        self.get(state.slot())
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
