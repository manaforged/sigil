use crate::style::StyleAtom;

kittest_pub! {
    /// Interaction state for a style variant.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    #[allow(
        clippy::struct_excessive_bools,
        reason = "intentional, reviewed trade-off"
    )]
    pub struct StyleState {
        /// Whether the node is disabled; this wins over every other state.
        pub disabled: bool,
        /// Whether the pointer is over the node.
        pub hovered: bool,
        /// Whether the node is being pressed.
        pub pressed: bool,
        /// Whether the node has focus.
        pub focused: bool,
    }
}

kittest_pub! {
    #[doc(hidden)]
    /// Names the interaction state that selects state style atoms.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub enum StateSlot {
        /// No interaction: not disabled, pressed, hovered, or focused.
        #[default]
        Rest,
        /// The node has focus and is not hovered, pressed, or disabled.
        Focused,
        /// The pointer is over the node, and the node is not pressed or disabled.
        Hovered,
        /// The node is being pressed and is not disabled.
        Pressed,
        /// The node is disabled; this slot wins over the others.
        Disabled,
    }
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

    pub(crate) fn into_each(self) -> [(StateSlot, T); 5] {
        [
            (StateSlot::Rest, self.rest),
            (StateSlot::Focused, self.focused),
            (StateSlot::Hovered, self.hovered),
            (StateSlot::Pressed, self.pressed),
            (StateSlot::Disabled, self.disabled),
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
    kittest_pub! {
        /// The slot that applies: disabled first, then pressed, hovered, focused, and rest.
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
}

kittest_pub! {
    /// Atoms per interaction state.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct StateStyle {
        slots: PerState<&'static [StyleAtom]>,
    }
}

impl StateStyle {
    kittest_pub! {
        /// A state style with no atoms.
        pub const EMPTY: Self = Self {
            slots: PerState::new(&[], &[], &[], &[], &[]),
        };
    }

    kittest_pub! {
        /// Creates a state style from the atoms of each state.
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
    }

    kittest_pub! {
        /// The atoms of `slot`.
        #[must_use]
        pub const fn get(self, slot: StateSlot) -> &'static [StyleAtom] {
            self.slots.at(slot)
        }
    }

    kittest_pub! {
        /// Returns the atoms that apply in `state`.
        #[must_use]
        pub const fn resolve(self, state: StyleState) -> &'static [StyleAtom] {
            self.get(state.slot())
        }
    }
}

#[cfg(feature = "kittest")]
impl StateStyle {
    /// Replaces the atoms of `slot`.
    #[must_use]
    pub const fn slot(mut self, slot: StateSlot, atoms: &'static [StyleAtom]) -> Self {
        *self.slots.get_mut(slot) = atoms;
        self
    }
}

impl Default for StateStyle {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Base atoms plus `StateStyle`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StyleVariant {
    base: &'static [StyleAtom],
    states: StateStyle,
}

impl StyleVariant {
    /// Creates a style variant.
    #[must_use]
    pub(crate) const fn new(base: &'static [StyleAtom], states: StateStyle) -> Self {
        Self { base, states }
    }

    /// Returns the base.
    #[must_use]
    pub(crate) const fn base(self) -> &'static [StyleAtom] {
        self.base
    }

    /// Returns the atoms this variant applies in `state`.
    #[must_use]
    pub(crate) const fn state(self, state: StyleState) -> &'static [StyleAtom] {
        self.states.resolve(state)
    }
}
