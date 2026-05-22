//! Struct-style builder API.

use smallvec::SmallVec;

use crate::{
    AlignKind, ColorAtom, Dimension, DisplayKind, FlexDir, JustifyKind, MillisU16, OverflowKind,
    Px, Rgba8, StyleAtom, TransitionId,
};

/// Chainable struct API for building style atom lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    atoms: SmallVec<[StyleAtom; 16]>,
    rest_atoms: SmallVec<[StyleAtom; 4]>,
    hover_atoms: SmallVec<[StyleAtom; 4]>,
    focus_atoms: SmallVec<[StyleAtom; 4]>,
    pressed_atoms: SmallVec<[StyleAtom; 4]>,
    disabled_atoms: SmallVec<[StyleAtom; 4]>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            atoms: SmallVec::new(),
            rest_atoms: SmallVec::new(),
            hover_atoms: SmallVec::new(),
            focus_atoms: SmallVec::new(),
            pressed_atoms: SmallVec::new(),
            disabled_atoms: SmallVec::new(),
        }
    }
}

impl Style {
    /// Create an empty style.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the built atoms.
    #[must_use]
    pub fn as_atoms(&self) -> &[StyleAtom] {
        &self.atoms
    }

    /// Consume this style into its atom storage.
    #[must_use]
    pub fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.atoms
    }

    /// Borrow rest-slot overlay atoms.
    #[must_use]
    pub fn rest_atoms(&self) -> &[StyleAtom] {
        &self.rest_atoms
    }

    /// Borrow hover-slot overlay atoms.
    #[must_use]
    pub fn hover_atoms(&self) -> &[StyleAtom] {
        &self.hover_atoms
    }

    /// Borrow focus-slot overlay atoms.
    #[must_use]
    pub fn focus_atoms(&self) -> &[StyleAtom] {
        &self.focus_atoms
    }

    /// Borrow pressed-slot overlay atoms.
    #[must_use]
    pub fn pressed_atoms(&self) -> &[StyleAtom] {
        &self.pressed_atoms
    }

    /// Borrow disabled-slot overlay atoms.
    #[must_use]
    pub fn disabled_atoms(&self) -> &[StyleAtom] {
        &self.disabled_atoms
    }

    /// True when any base atoms or state-slot overlays have been pushed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty() && !self.has_state_overlays()
    }

    /// True when any state-slot overlay has at least one atom.
    #[must_use]
    pub fn has_state_overlays(&self) -> bool {
        !self.rest_atoms.is_empty()
            || !self.hover_atoms.is_empty()
            || !self.focus_atoms.is_empty()
            || !self.pressed_atoms.is_empty()
            || !self.disabled_atoms.is_empty()
    }

    /// Replace the rest-slot overlay with `atoms` (last-call-wins per DR-049).
    pub(crate) fn set_rest(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.rest_atoms = atoms.into_iter().collect();
    }

    /// Replace the hover-slot overlay with `atoms` (last-call-wins per DR-049).
    pub(crate) fn set_hover(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.hover_atoms = atoms.into_iter().collect();
    }

    /// Replace the focus-slot overlay with `atoms`.
    pub(crate) fn set_focus(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.focus_atoms = atoms.into_iter().collect();
    }

    /// Replace the pressed-slot overlay with `atoms`.
    pub(crate) fn set_pressed(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.pressed_atoms = atoms.into_iter().collect();
    }

    /// Replace the disabled-slot overlay with `atoms`.
    pub(crate) fn set_disabled(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.disabled_atoms = atoms.into_iter().collect();
    }

    /// Drain the state-slot overlays into a [`StyleOverlay`].
    ///
    /// Use this in a widget builder's `into_node_builder()` to lift
    /// user-supplied `.rest()` / `.hover()` / `.focus()` /
    /// `.pressed()` / `.disabled()` atoms out of the accumulator
    /// before the base atoms attach to the node. Ownership transfers
    /// to the caller — no leaking, no `'static` laundering. The
    /// resulting overlay attaches to a node via
    /// [`StyleBinding::set_state_overlay`](crate::StyleBinding::set_state_overlay).
    #[must_use]
    pub fn take_state_overlay(&mut self) -> crate::StyleOverlay {
        crate::StyleOverlay::from_slots(
            std::mem::take(&mut self.rest_atoms),
            std::mem::take(&mut self.hover_atoms),
            std::mem::take(&mut self.focus_atoms),
            std::mem::take(&mut self.pressed_atoms),
            std::mem::take(&mut self.disabled_atoms),
        )
    }

    /// Append `atom` to the base list — used by the `Styled` trait's
    /// default impls.
    pub(crate) fn push_atom(&mut self, atom: StyleAtom) {
        self.atoms.push(atom);
    }

    /// Set uniform padding in logical pixels.
    #[must_use]
    pub fn padding(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingAll(Px(px)))
    }

    /// Set horizontal padding in logical pixels.
    #[must_use]
    pub fn px(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingX(Px(px)))
    }

    /// Set vertical padding in logical pixels.
    #[must_use]
    pub fn py(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingY(Px(px)))
    }

    /// Set top padding in logical pixels.
    #[must_use]
    pub fn pt(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingTop(Px(px)))
    }

    /// Set right padding in logical pixels.
    #[must_use]
    pub fn pr(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingRight(Px(px)))
    }

    /// Set bottom padding in logical pixels.
    #[must_use]
    pub fn pb(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingBottom(Px(px)))
    }

    /// Set left padding in logical pixels.
    #[must_use]
    pub fn pl(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingLeft(Px(px)))
    }

    /// Set uniform margin in logical pixels.
    #[must_use]
    pub fn margin(self, px: i16) -> Self {
        self.push(StyleAtom::MarginAll(Px(px)))
    }

    /// Set horizontal margin in logical pixels.
    #[must_use]
    pub fn mx(self, px: i16) -> Self {
        self.push(StyleAtom::MarginX(Px(px)))
    }

    /// Set vertical margin in logical pixels.
    #[must_use]
    pub fn my(self, px: i16) -> Self {
        self.push(StyleAtom::MarginY(Px(px)))
    }

    /// Set top margin in logical pixels.
    #[must_use]
    pub fn mt(self, px: i16) -> Self {
        self.push(StyleAtom::MarginTop(Px(px)))
    }

    /// Set right margin in logical pixels.
    #[must_use]
    pub fn mr(self, px: i16) -> Self {
        self.push(StyleAtom::MarginRight(Px(px)))
    }

    /// Set bottom margin in logical pixels.
    #[must_use]
    pub fn mb(self, px: i16) -> Self {
        self.push(StyleAtom::MarginBottom(Px(px)))
    }

    /// Set left margin in logical pixels.
    #[must_use]
    pub fn ml(self, px: i16) -> Self {
        self.push(StyleAtom::MarginLeft(Px(px)))
    }

    /// Set flex/grid gap in logical pixels.
    #[must_use]
    pub fn gap(self, px: i16) -> Self {
        self.push(StyleAtom::Gap(Px(px)))
    }

    /// Set preferred width.
    #[must_use]
    pub fn width(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::Width(dimension))
    }

    /// Set preferred height.
    #[must_use]
    pub fn height(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::Height(dimension))
    }

    /// Set minimum width.
    #[must_use]
    pub fn min_width(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::MinWidth(dimension))
    }

    /// Set maximum width.
    #[must_use]
    pub fn max_width(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::MaxWidth(dimension))
    }

    /// Set display mode.
    #[must_use]
    pub fn display(self, display: DisplayKind) -> Self {
        self.push(StyleAtom::Display(display))
    }

    /// Set flex direction.
    #[must_use]
    pub fn flex_direction(self, direction: FlexDir) -> Self {
        self.push(StyleAtom::FlexDirection(direction))
    }

    /// Set cross-axis alignment.
    #[must_use]
    pub fn align_items(self, align: AlignKind) -> Self {
        self.push(StyleAtom::AlignItems(align))
    }

    /// Set main-axis distribution.
    #[must_use]
    pub fn justify_content(self, justify: JustifyKind) -> Self {
        self.push(StyleAtom::JustifyContent(justify))
    }

    /// Set flex grow encoded as `value * 1000`.
    #[must_use]
    pub fn flex_grow(self, grow: u16) -> Self {
        self.push(StyleAtom::FlexGrow(grow))
    }

    /// Set flex shrink encoded as `value * 1000`.
    #[must_use]
    pub fn flex_shrink(self, shrink: u16) -> Self {
        self.push(StyleAtom::FlexShrink(shrink))
    }

    /// Set a concrete background colour.
    #[must_use]
    pub fn bg(self, color: Rgba8) -> Self {
        self.fill(ColorAtom::Rgba8(color))
    }

    /// Set a background colour atom.
    #[must_use]
    pub fn fill(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::BackgroundColor(color))
    }

    /// Set a background colour theme token.
    #[must_use]
    pub fn background_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::BackgroundThemeRef(token))
    }

    /// Set a text colour atom.
    #[must_use]
    pub fn text_color(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::TextColor(color))
    }

    /// Set a text colour theme token.
    #[must_use]
    pub fn text_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::TextThemeRef(token))
    }

    /// Set border radius in logical pixels.
    #[must_use]
    pub fn radius(self, px: i16) -> Self {
        self.push(StyleAtom::BorderRadius(Px(px)))
    }

    /// Set opacity encoded as `value * 10000`.
    #[must_use]
    pub fn opacity(self, opacity: u16) -> Self {
        self.push(StyleAtom::Opacity(opacity))
    }

    /// Set font size in logical pixels.
    #[must_use]
    pub fn font_size(self, px: i16) -> Self {
        self.push(StyleAtom::FontSize(Px(px)))
    }

    /// Set font weight.
    #[must_use]
    pub fn font_weight(self, weight: u16) -> Self {
        self.push(StyleAtom::FontWeight(weight))
    }

    /// Set font family. Accepts a `&'static str` family name resolved
    /// by the text engine (e.g. `"sans"`, `"mono"`, `"serif"`).
    #[must_use]
    pub fn font_family(self, name: &'static str) -> Self {
        self.push(StyleAtom::FontFamily(name))
    }

    /// Set line height in logical pixels.
    #[must_use]
    pub fn line_height(self, px: i16) -> Self {
        self.push(StyleAtom::LineHeight(Px(px)))
    }

    /// Store a deferred transition preset.
    #[must_use]
    pub fn transition(self, transition: TransitionId) -> Self {
        self.push(StyleAtom::Transition(transition))
    }

    /// Store a deferred transition duration.
    #[must_use]
    pub fn duration(self, duration: MillisU16) -> Self {
        self.push(StyleAtom::TransitionDuration(duration))
    }

    /// Store an unsupported theme-style token for diagnostics.
    #[must_use]
    pub fn unknown_theme_ref(self, token: &'static str) -> Self {
        self.push(StyleAtom::UnknownThemeRef(token))
    }

    /// Store an overflow rule on the inline (width) axis.
    #[must_use]
    pub fn overflow_x(self, kind: OverflowKind) -> Self {
        self.push(StyleAtom::OverflowX(kind))
    }

    /// Store an overflow rule on the block (height) axis.
    #[must_use]
    pub fn overflow_y(self, kind: OverflowKind) -> Self {
        self.push(StyleAtom::OverflowY(kind))
    }

    fn push(mut self, atom: StyleAtom) -> Self {
        self.atoms.push(atom);
        self
    }
}
