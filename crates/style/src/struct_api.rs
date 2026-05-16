//! Struct-style builder API.

use smallvec::SmallVec;

use crate::{
    AlignKind, ColorAtom, Dimension, DisplayKind, FlexDir, JustifyKind, MillisU16, Px, Rgba8,
    StyleAtom, TransitionId,
};

/// Chainable struct API for building style atom lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    atoms: SmallVec<[StyleAtom; 16]>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            atoms: SmallVec::new(),
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
        self.background_color(ColorAtom::Rgba8(color))
    }

    /// Set a background colour atom.
    #[must_use]
    pub fn background_color(self, color: ColorAtom) -> Self {
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
    pub fn transition_duration(self, duration: MillisU16) -> Self {
        self.push(StyleAtom::TransitionDuration(duration))
    }

    /// Store an unsupported theme-style token for diagnostics.
    #[must_use]
    pub fn unknown_theme_ref(self, token: &'static str) -> Self {
        self.push(StyleAtom::UnknownThemeRef(token))
    }

    fn push(mut self, atom: StyleAtom) -> Self {
        self.atoms.push(atom);
        self
    }
}
