use smallvec::SmallVec;

use crate::style::{
    AlignKind, ColorAtom, ColorKey, Dimension, DisplayKind, Elevation, FlexDir, FlexWrapKind,
    JustifyKind, Px, StateSlot, StyleAtom,
    parser::{StatefulToken, parse_stateful_token},
};

mod paint;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Style {
    atoms: SmallVec<[StyleAtom; 16]>,
    rest_atoms: SmallVec<[StyleAtom; 4]>,
    hover_atoms: SmallVec<[StyleAtom; 4]>,
    focus_atoms: SmallVec<[StyleAtom; 4]>,
    pressed_atoms: SmallVec<[StyleAtom; 4]>,
    disabled_atoms: SmallVec<[StyleAtom; 4]>,
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
                Err(err) => crate::style::builder_ext::diagnostic::class_parse(input, &err),
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
    pub fn border_theme(self, token: impl Into<ColorKey>) -> Self {
        self.push(StyleAtom::BorderThemeRef(token.into()))
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

    fn push(mut self, atom: StyleAtom) -> Self {
        self.atoms.push(atom);
        self
    }
}
