use smallvec::SmallVec;

use crate::style::{
    AlignKind, ColorAtom, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind, JustifyKind,
    MillisU16, OverflowKind, Px, Rgba8, StateSlot, StyleAtom, TransitionId,
    parser::{StatefulToken, parse_stateful_token},
};

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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn as_atoms(&self) -> &[StyleAtom] {
        &self.atoms
    }

    #[must_use]
    pub fn into_atoms(self) -> SmallVec<[StyleAtom; 16]> {
        self.atoms
    }

    #[must_use]
    pub fn class(mut self, input: &str) -> Self {
        for token in input.split(' ').filter(|token| !token.is_empty()) {
            match parse_stateful_token(token) {
                Ok(parsed) => self.absorb_token(parsed),
                Err(err) => {
                    crate::style::merge::bump_unknown_class_count();
                    tracing::warn!(
                        target: "sigil_style::class",
                        input,
                        error = %err,
                        "unknown class token ignored",
                    );
                }
            }
        }
        self
    }

    fn absorb_token(&mut self, parsed: StatefulToken) {
        match parsed.slot {
            None => self.atoms.extend(parsed.atoms),
            Some(StateSlot::Rest) => self.rest_atoms.extend(parsed.atoms),
            Some(StateSlot::Hovered) => self.hover_atoms.extend(parsed.atoms),
            Some(StateSlot::Focused) => self.focus_atoms.extend(parsed.atoms),
            Some(StateSlot::Pressed) => self.pressed_atoms.extend(parsed.atoms),
            Some(StateSlot::Disabled) => self.disabled_atoms.extend(parsed.atoms),
        }
    }

    #[must_use]
    pub fn rest_atoms(&self) -> &[StyleAtom] {
        &self.rest_atoms
    }

    #[must_use]
    pub fn hover_atoms(&self) -> &[StyleAtom] {
        &self.hover_atoms
    }

    #[must_use]
    pub fn focus_atoms(&self) -> &[StyleAtom] {
        &self.focus_atoms
    }

    #[must_use]
    pub fn pressed_atoms(&self) -> &[StyleAtom] {
        &self.pressed_atoms
    }

    #[must_use]
    pub fn disabled_atoms(&self) -> &[StyleAtom] {
        &self.disabled_atoms
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty() && !self.has_state_overlays()
    }

    #[must_use]
    pub fn has_state_overlays(&self) -> bool {
        !self.rest_atoms.is_empty()
            || !self.hover_atoms.is_empty()
            || !self.focus_atoms.is_empty()
            || !self.pressed_atoms.is_empty()
            || !self.disabled_atoms.is_empty()
    }

    pub(crate) fn set_rest(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.rest_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_hover(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.hover_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_focus(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.focus_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_pressed(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.pressed_atoms = atoms.into_iter().collect();
    }

    pub(crate) fn set_disabled(&mut self, atoms: SmallVec<[StyleAtom; 16]>) {
        self.disabled_atoms = atoms.into_iter().collect();
    }

    #[must_use]
    pub fn take_state_overlay(&mut self) -> crate::style::StyleOverlay {
        crate::style::StyleOverlay::from_slots(
            std::mem::take(&mut self.rest_atoms),
            std::mem::take(&mut self.hover_atoms),
            std::mem::take(&mut self.focus_atoms),
            std::mem::take(&mut self.pressed_atoms),
            std::mem::take(&mut self.disabled_atoms),
        )
    }

    pub(crate) fn push_atom(&mut self, atom: StyleAtom) {
        self.atoms.push(atom);
    }

    #[must_use]
    pub fn padding(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingAll(Px(px)))
    }

    #[must_use]
    pub fn px(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingX(Px(px)))
    }

    #[must_use]
    pub fn py(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingY(Px(px)))
    }

    #[must_use]
    pub fn pt(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingTop(Px(px)))
    }

    #[must_use]
    pub fn pr(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingRight(Px(px)))
    }

    #[must_use]
    pub fn pb(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingBottom(Px(px)))
    }

    #[must_use]
    pub fn pl(self, px: i16) -> Self {
        self.push(StyleAtom::PaddingLeft(Px(px)))
    }

    #[must_use]
    pub fn margin(self, px: i16) -> Self {
        self.push(StyleAtom::MarginAll(Px(px)))
    }

    #[must_use]
    pub fn mx(self, px: i16) -> Self {
        self.push(StyleAtom::MarginX(Px(px)))
    }

    #[must_use]
    pub fn my(self, px: i16) -> Self {
        self.push(StyleAtom::MarginY(Px(px)))
    }

    #[must_use]
    pub fn mt(self, px: i16) -> Self {
        self.push(StyleAtom::MarginTop(Px(px)))
    }

    #[must_use]
    pub fn mr(self, px: i16) -> Self {
        self.push(StyleAtom::MarginRight(Px(px)))
    }

    #[must_use]
    pub fn mb(self, px: i16) -> Self {
        self.push(StyleAtom::MarginBottom(Px(px)))
    }

    #[must_use]
    pub fn ml(self, px: i16) -> Self {
        self.push(StyleAtom::MarginLeft(Px(px)))
    }

    #[must_use]
    pub fn gap(self, px: i16) -> Self {
        self.push(StyleAtom::Gap(Px(px)))
    }

    #[must_use]
    pub fn gap_x(self, px: i16) -> Self {
        self.push(StyleAtom::GapX(Px(px)))
    }

    #[must_use]
    pub fn gap_y(self, px: i16) -> Self {
        self.push(StyleAtom::GapY(Px(px)))
    }

    #[must_use]
    pub fn flex_wrap(self, wrap: FlexWrapKind) -> Self {
        self.push(StyleAtom::FlexWrap(wrap))
    }

    #[must_use]
    pub fn width(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::Width(dimension))
    }

    #[must_use]
    pub fn height(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::Height(dimension))
    }

    #[must_use]
    pub fn min_width(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::MinWidth(dimension))
    }

    #[must_use]
    pub fn max_width(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::MaxWidth(dimension))
    }

    #[must_use]
    pub fn min_height(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::MinHeight(dimension))
    }

    #[must_use]
    pub fn max_height(self, dimension: Dimension) -> Self {
        self.push(StyleAtom::MaxHeight(dimension))
    }

    #[must_use]
    pub fn border_width(self, width: i16) -> Self {
        self.push(StyleAtom::BorderWidth(Px(width)))
    }

    #[must_use]
    pub fn border_top(self, width: i16) -> Self {
        self.push(StyleAtom::BorderTop(Px(width)))
    }

    #[must_use]
    pub fn border_right(self, width: i16) -> Self {
        self.push(StyleAtom::BorderRight(Px(width)))
    }

    #[must_use]
    pub fn border_bottom(self, width: i16) -> Self {
        self.push(StyleAtom::BorderBottom(Px(width)))
    }

    #[must_use]
    pub fn border_left(self, width: i16) -> Self {
        self.push(StyleAtom::BorderLeft(Px(width)))
    }

    #[must_use]
    pub fn border_color(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::BorderColor(color))
    }

    #[must_use]
    pub fn shadow(self, elevation: Elevation) -> Self {
        self.push(StyleAtom::Shadow(elevation))
    }

    #[must_use]
    pub fn border_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::BorderThemeRef(token))
    }

    #[must_use]
    pub fn display(self, display: DisplayKind) -> Self {
        self.push(StyleAtom::Display(display))
    }

    #[must_use]
    pub fn flex_direction(self, direction: FlexDir) -> Self {
        self.push(StyleAtom::FlexDirection(direction))
    }

    #[must_use]
    pub fn align_items(self, align: AlignKind) -> Self {
        self.push(StyleAtom::AlignItems(align))
    }

    #[must_use]
    pub fn justify_content(self, justify: JustifyKind) -> Self {
        self.push(StyleAtom::JustifyContent(justify))
    }

    #[must_use]
    pub fn flex_grow(self, grow: u16) -> Self {
        self.push(StyleAtom::FlexGrow(grow))
    }

    #[must_use]
    pub fn grid_columns(self, n: u16) -> Self {
        self.push(StyleAtom::GridColumns(n))
    }

    #[must_use]
    pub fn grid_rows(self, n: u16) -> Self {
        self.push(StyleAtom::GridRows(n))
    }

    #[must_use]
    pub fn flex_shrink(self, shrink: u16) -> Self {
        self.push(StyleAtom::FlexShrink(shrink))
    }

    #[must_use]
    pub fn bg(self, color: Rgba8) -> Self {
        self.fill(ColorAtom::Rgba8(color))
    }

    #[must_use]
    pub fn fill(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::BackgroundColor(color))
    }

    #[must_use]
    pub fn background_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::BackgroundThemeRef(token))
    }

    #[must_use]
    pub fn text_color(self, color: ColorAtom) -> Self {
        self.push(StyleAtom::TextColor(color))
    }

    #[must_use]
    pub fn text_theme(self, token: &'static str) -> Self {
        self.push(StyleAtom::TextThemeRef(token))
    }

    #[must_use]
    pub fn radius(self, px: i16) -> Self {
        self.push(StyleAtom::BorderRadius(Px(px)))
    }

    #[must_use]
    #[track_caller]
    pub fn opacity(self, value: f32) -> Self {
        debug_assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "opacity must be a finite value in 0.0..=1.0, got {value}",
        );
        let clamped = value.clamp(0.0, 1.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "values are bounded to the target range before the cast"
        )]
        let encoded = (clamped * 10_000.0).round() as u16;
        self.push(StyleAtom::Opacity(encoded))
    }

    #[doc(hidden)]
    #[must_use]
    pub fn opacity_raw(self, opacity: u16) -> Self {
        self.push(StyleAtom::Opacity(opacity))
    }

    #[must_use]
    pub fn font_size(self, px: i16) -> Self {
        self.push(StyleAtom::FontSize(Px(px)))
    }

    #[must_use]
    pub fn font_weight(self, weight: u16) -> Self {
        self.push(StyleAtom::FontWeight(weight))
    }

    #[must_use]
    pub fn font_italic(self, italic: bool) -> Self {
        self.push(StyleAtom::FontItalic(italic))
    }

    #[must_use]
    pub fn font_family(self, name: &'static str) -> Self {
        self.push(StyleAtom::FontFamily(name))
    }

    #[must_use]
    pub fn line_height(self, px: i16) -> Self {
        self.push(StyleAtom::LineHeight(Px(px)))
    }

    #[must_use]
    pub fn letter_spacing_em(self, em: f32) -> Self {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "values are bounded to the target range before the cast"
        )]
        let thousandths = (em * 1000.0).round().clamp(-2000.0, 2000.0) as i16;
        self.push(StyleAtom::LetterSpacing(thousandths))
    }

    #[must_use]
    pub fn tabular_nums(self, on: bool) -> Self {
        self.push(StyleAtom::TabularNums(on))
    }

    #[must_use]
    pub fn text_decoration(self, decoration: crate::style::TextDecoration) -> Self {
        self.push(StyleAtom::TextDecoration(decoration))
    }

    #[must_use]
    pub fn line_through(self) -> Self {
        self.text_decoration(crate::style::TextDecoration::LineThrough)
    }

    #[must_use]
    pub fn underline(self) -> Self {
        self.text_decoration(crate::style::TextDecoration::Underline)
    }

    #[must_use]
    pub fn transition(self, transition: TransitionId) -> Self {
        self.push(StyleAtom::Transition(transition))
    }

    #[must_use]
    pub fn duration(self, duration: MillisU16) -> Self {
        self.push(StyleAtom::TransitionDuration(duration))
    }

    #[must_use]
    pub fn unknown_theme_ref(self, token: &'static str) -> Self {
        self.push(StyleAtom::UnknownThemeRef(token))
    }

    #[must_use]
    pub fn overflow_x(self, kind: OverflowKind) -> Self {
        self.push(StyleAtom::OverflowX(kind))
    }

    #[must_use]
    pub fn overflow_y(self, kind: OverflowKind) -> Self {
        self.push(StyleAtom::OverflowY(kind))
    }

    fn push(mut self, atom: StyleAtom) -> Self {
        self.atoms.push(atom);
        self
    }
}
