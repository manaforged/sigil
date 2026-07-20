//! `StateVariants` super-trait — `.hover` / `.focus` / `.pressed` /
//! `.disabled` slot overlays.
//!
//! Bounded on [`StateHost`] so [`StateStyleBuilder`] (the closure
//! parameter) deliberately does NOT receive these methods, which makes
//! nested state-variant closures (`.hover(|s| s.hover(...))`) a compile
//! error per DR-049 Decision 4.

use super::{StateHost, StateStyleBuilder, StyleAccess};

/// State-slot overlay methods — gated on [`StateHost`].
///
/// Implementors of [`StyleAccess`] that also implement [`StateHost`]
/// receive `.hover`, `.focus`, `.pressed`, and `.disabled`.
/// [`StateStyleBuilder`] deliberately does NOT implement [`StateHost`],
/// so nesting these closures fails to compile.
pub trait StateVariants: StyleAccess + StateHost {
    /// Configure the rest-slot overlay via a closure.
    ///
    /// Rest atoms apply when no other state is active. The closure
    /// receives a [`StateStyleBuilder`] that responds to the same
    /// fluent vocabulary as the other state methods. Calling `.rest`
    /// twice **overwrites** the slot (last-call-wins per DR-049
    /// Decision 8).
    ///
    /// DS2.5 — this slot lets you override the widget's baked
    /// rest-state atoms without forking the widget. The user atoms
    /// resolve last, so per-property last-wins gives them priority
    /// over the widget's static rest atoms.
    #[must_use]
    fn rest<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_rest(built.into_atoms());
        self
    }

    /// Configure the hover-slot overlay via a closure.
    ///
    /// The closure receives a [`StateStyleBuilder`] that responds to the
    /// same fluent vocabulary. Calling `.hover` twice **overwrites** the
    /// hover slot (last-call-wins per DR-049 Decision 8).
    #[must_use]
    fn hover<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_hover(built.into_atoms());
        self
    }

    /// Configure the focus-slot overlay via a closure.
    #[must_use]
    fn focus<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_focus(built.into_atoms());
        self
    }

    /// Configure the pressed-slot overlay via a closure.
    #[must_use]
    fn pressed<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_pressed(built.into_atoms());
        self
    }

    /// Configure the disabled-slot overlay via a closure.
    ///
    /// Named `disabled_style` (5/5 Wave 1, D-U2) so behavioral
    /// `.disabled(Signal<bool>)` — the far more common call — keeps the
    /// short name without a method-resolution collision.
    #[must_use]
    fn disabled_style<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_disabled(built.into_atoms());
        self
    }
}

impl<T: StyleAccess + StateHost> StateVariants for T {}
