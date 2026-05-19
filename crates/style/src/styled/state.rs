//! `StateStyleBuilder` — the closure parameter for `.hover` /
//! `.focus` / `.pressed` / `.disabled` — plus the `StateHost`
//! marker that seals nested state-variant calls.

use smallvec::SmallVec;

use super::StyleAccess;
use crate::{Style, StyleAtom};

/// Marker trait sealing state-variant methods to outer builders only.
///
/// Implementors of [`Styled`] that also implement this marker can call
/// `.hover`, `.focus`, `.pressed`, and `.disabled`. [`StateStyleBuilder`]
/// deliberately does NOT implement this marker, which makes nested
/// state-variant closures (`.hover(|s| s.hover(|s2| ...))`) a compile
/// error per DR-049 Decision 4.
///
/// Nested state-variant closures fail to compile:
///
/// ```compile_fail
/// use sigil_kernel::container;
/// use sigil_style::{Styled, StyledNodeBuilder};
/// // `s` inside the closure is a StateStyleBuilder, which does not
/// // implement StateHost — so `.hover` is not callable.
/// let _ = container().styled().hover(|s| s.hover(|inner| inner));
/// ```
pub trait StateHost {}

/// Closure parameter for [`super::StateVariants::hover`] /
/// [`super::StateVariants::focus`] / [`super::StateVariants::pressed`] /
/// [`super::StateVariants::disabled`].
///
/// Implements [`StyleAccess`] so the same fluent vocabulary works inside
/// the closure (every per-category super-trait is a blanket impl over
/// `StyleAccess`). Does NOT implement [`StateHost`], so nested
/// state-variant calls are a compile error.
pub struct StateStyleBuilder {
    style: Style,
}

impl StateStyleBuilder {
    /// Create a fresh empty state-style builder.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            style: Style::new(),
        }
    }

    /// Consume the builder into its atom list for committing to a slot.
    #[must_use]
    pub(crate) fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.style.into_atoms()
    }
}

impl StyleAccess for StateStyleBuilder {
    fn style(&mut self) -> &mut Style {
        &mut self.style
    }
}
