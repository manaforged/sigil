//! `StateStyleBuilder` — the closure parameter for `.hover` /
//! `.focus` / `.pressed` / `.disabled` — plus the `StateHost`
//! marker that seals nested state-variant calls.

use smallvec::SmallVec;

use super::StyleAccess;
use crate::{Style, StyleAtom};

/// Marker trait sealing state-variant methods to outer builders only.
///
/// Implementors of [`Styled`] that also implement this marker can call
/// `.rest`, `.hover`, `.focus`, `.pressed`, and `.disabled`.
/// [`StateStyleBuilder`] deliberately does NOT implement this marker,
/// which makes nested state-variant closures
/// (`.hover(|s| s.hover(|s2| ...))`) a compile error per DR-049
/// Decision 4.
///
/// # Implementation contract (DS2.5)
///
/// **A widget builder that `impl StateHost` MUST drain the state-slot
/// overlay accumulator into its node binding.** The reactive surface
/// already wrote the user's `.hover()` / `.pressed()` / etc. atoms
/// into the builder's `Style` accumulator — without an explicit
/// `take_state_overlay()` + `stateful_style()` call those atoms drop
/// silently with no compile error, no runtime warning, no painted
/// effect. The canonical pattern lives in
/// `crates/sigil/src/widgets/button.rs`:
///
/// ```ignore
/// fn into_node_builder(mut self) -> NodeBuilder {
///     let user_overlay = self.style_accumulator.take_state_overlay();
///     container()
///         .class(variant.base())
///         .stateful_style(variant, user_overlay, move || state_fn())
///         // ...
/// }
/// ```
///
/// Widgets that don't follow this pattern silently lie about their
/// state-overlay support. If your widget genuinely doesn't paint
/// interaction states (e.g. a static panel), do NOT
/// `impl StateHost for YourBuilder` — leave it out so call-site
/// `.hover()` produces a clear compile error instead of a silent
/// drop.
///
/// Nested state-variant closures fail to compile:
///
/// ```compile_fail
/// use kernel::container;
/// use style::{Styled, StyledNodeBuilder};
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}
