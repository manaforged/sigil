//! Style atom resolver.

use crate::{
    AnimatorOverlay, ColorAtom, ColorAtomKind, NumericAtomKind, Px, RenderStyle, Rgba8, StyleAtom,
    TextLineHeight, TextStyle, Theme, Tone,
};

/// Resolves style atoms against a theme + per-node tone.
#[derive(Debug, Clone, Copy)]
pub struct StyleEngine<'theme> {
    theme: &'theme Theme,
    tone: Tone,
}

impl<'theme> StyleEngine<'theme> {
    /// Create a resolver over a theme snapshot, defaulting to the
    /// neutral tone.
    #[must_use]
    pub const fn new(theme: &'theme Theme) -> Self {
        Self::with_tone(theme, Tone::Neutral)
    }

    /// Create a resolver over a theme snapshot bound to `tone`.
    ///
    /// The tone selects which scale on the theme
    /// [`BgToneStep`](StyleAtom::BgToneStep) /
    /// [`FgToneStep`](StyleAtom::FgToneStep) /
    /// [`BorderToneStep`](StyleAtom::BorderToneStep) atoms resolve
    /// against.
    #[must_use]
    pub const fn with_tone(theme: &'theme Theme, tone: Tone) -> Self {
        Self { theme, tone }
    }

    /// Apply theme-backed visual atoms into the render sink.
    pub fn apply(&self, atoms: &[StyleAtom], render: &mut RenderStyle) {
        for atom in atoms {
            (*self).apply_atom(*atom, render);
        }
    }

    /// Apply theme-backed visual atoms, consulting `overlay` for
    /// animator-driven numeric values.
    ///
    /// W5.1b — when `overlay.value_for(kind)` returns `Some(v)`, the
    /// resolved render style uses `v` instead of the literal atom
    /// value. Used by the renderer when a binding has attached
    /// animators (DR-053 §5).
    pub fn apply_with_overlay(
        &self,
        atoms: &[StyleAtom],
        overlay: &dyn AnimatorOverlay,
        render: &mut RenderStyle,
    ) {
        for atom in atoms {
            (*self).apply_atom_overlaid(*atom, overlay, render);
        }
    }

    /// Apply typography atoms into the text style sink.
    #[allow(clippy::unused_self)]
    pub fn apply_text(&self, atoms: &[StyleAtom], text: &mut TextStyle) {
        apply_text_atoms(atoms, text);
    }

    /// Resolve a tone-step atom through this engine's theme + tone.
    #[must_use]
    const fn tone_step(&self, step: u8) -> Rgba8 {
        self.theme.scale(self.tone.scale_role()).step(step)
    }

    /// Resolve a colour atom through this engine's theme.
    #[must_use]
    pub fn resolve_color(&self, color: ColorAtom) -> Rgba8 {
        match color {
            ColorAtom::Rgba8(color) => color,
            ColorAtom::ThemeRef(token) => self.theme.resolve_color(token),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "exhaustive match on StyleAtom — the long no-op arm chain is intentional; a wildcard would silently swallow new atom kinds"
    )]
    fn apply_atom(self, atom: StyleAtom, render: &mut RenderStyle) {
        match atom {
            StyleAtom::BackgroundColor(color) => {
                render.background_color = Some(self.resolve_color(color));
            }
            StyleAtom::BackgroundThemeRef(token) => {
                render.background_color = Some(self.theme.resolve_color(token));
            }
            StyleAtom::TextColor(color) => render.text_color = Some(self.resolve_color(color)),
            StyleAtom::TextThemeRef(token) => {
                render.text_color = Some(self.theme.resolve_color(token));
            }
            StyleAtom::BgToneStep(step) => render.background_color = Some(self.tone_step(step)),
            StyleAtom::FgToneStep(step) => render.text_color = Some(self.tone_step(step)),
            StyleAtom::FgOnSolid => {
                render.text_color = Some(self.theme.scale(self.tone.scale_role()).on_solid);
            }
            StyleAtom::BorderRadius(radius) => render.border_radius = Some(radius),
            StyleAtom::BorderWidth(width) => render.border_width = Some(width),
            StyleAtom::BorderColor(color) => render.border_color = Some(self.resolve_color(color)),
            StyleAtom::BorderThemeRef(token) => {
                render.border_color = Some(self.theme.resolve_color(token));
            }
            StyleAtom::BorderToneStep(step) => render.border_color = Some(self.tone_step(step)),
            StyleAtom::Shadow(elevation) => render.shadow = Some(elevation),
            StyleAtom::Opacity(opacity) => render.opacity = opacity,
            StyleAtom::PaddingAll(_)
            | StyleAtom::PaddingX(_)
            | StyleAtom::PaddingY(_)
            | StyleAtom::PaddingTop(_)
            | StyleAtom::PaddingRight(_)
            | StyleAtom::PaddingBottom(_)
            | StyleAtom::PaddingLeft(_)
            | StyleAtom::MarginAll(_)
            | StyleAtom::MarginX(_)
            | StyleAtom::MarginY(_)
            | StyleAtom::MarginTop(_)
            | StyleAtom::MarginRight(_)
            | StyleAtom::MarginBottom(_)
            | StyleAtom::MarginLeft(_)
            | StyleAtom::Gap(_)
            | StyleAtom::GapX(_)
            | StyleAtom::GapY(_)
            | StyleAtom::Width(_)
            | StyleAtom::Height(_)
            | StyleAtom::MinWidth(_)
            | StyleAtom::MaxWidth(_)
            | StyleAtom::MinHeight(_)
            | StyleAtom::MaxHeight(_)
            | StyleAtom::Display(_)
            | StyleAtom::FlexDirection(_)
            | StyleAtom::FlexWrap(_)
            | StyleAtom::AlignItems(_)
            | StyleAtom::JustifyContent(_)
            | StyleAtom::FlexGrow(_)
            | StyleAtom::FlexShrink(_)
            | StyleAtom::GridColumns(_)
            | StyleAtom::GridRows(_)
            | StyleAtom::FontSize(_)
            | StyleAtom::FontWeight(_)
            | StyleAtom::FontFamily(_)
            | StyleAtom::LineHeight(_)
            | StyleAtom::OverflowX(_)
            | StyleAtom::OverflowY(_)
            | StyleAtom::Transition(_)
            | StyleAtom::TransitionDuration(_)
            | StyleAtom::UnknownThemeRef(_) => {}
        }
    }

    fn apply_atom_overlaid(
        self,
        atom: StyleAtom,
        overlay: &dyn AnimatorOverlay,
        render: &mut RenderStyle,
    ) {
        // Only the *render-visible* numeric atoms get overlaid here.
        // Layout-flowing atoms (padding, gap, font size, line height)
        // are owned by the layout engine; the overlay reaches those via
        // the layout-side seam landing in a follow-up phase.
        match atom {
            StyleAtom::Opacity(literal) => {
                let resolved = overlay
                    .value_for(NumericAtomKind::Opacity)
                    .map_or(literal, encode_opacity);
                render.opacity = resolved;
            }
            StyleAtom::BorderRadius(literal) => {
                let resolved = overlay
                    .value_for(NumericAtomKind::BorderRadius)
                    .map_or(literal, encode_px);
                render.border_radius = Some(resolved);
            }
            StyleAtom::BackgroundColor(literal) => {
                let resolved = overlay
                    .value_for_color(ColorAtomKind::Background)
                    .unwrap_or_else(|| self.resolve_color(literal));
                render.background_color = Some(resolved);
            }
            StyleAtom::BackgroundThemeRef(token) => {
                let resolved = overlay
                    .value_for_color(ColorAtomKind::Background)
                    .unwrap_or_else(|| self.theme.resolve_color(token));
                render.background_color = Some(resolved);
            }
            StyleAtom::TextColor(literal) => {
                let resolved = overlay
                    .value_for_color(ColorAtomKind::Text)
                    .unwrap_or_else(|| self.resolve_color(literal));
                render.text_color = Some(resolved);
            }
            StyleAtom::TextThemeRef(token) => {
                let resolved = overlay
                    .value_for_color(ColorAtomKind::Text)
                    .unwrap_or_else(|| self.theme.resolve_color(token));
                render.text_color = Some(resolved);
            }
            _ => self.apply_atom(atom, render),
        }
    }
}

/// Encode an animator-driven `f32` back into the `u16` representation
/// used by [`StyleAtom::Opacity`] / [`RenderStyle::opacity`]
/// (`value * 10_000` clamped to `0..=u16::MAX`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to 0..=u16::MAX before the cast — fits losslessly in u16"
)]
fn encode_opacity(value: f32) -> u16 {
    value.clamp(0.0, f32::from(u16::MAX)).round() as u16
}

/// Encode an animator-driven `f32` back into the `i16`-px representation
/// used by [`StyleAtom::BorderRadius`] / [`Px`] (rounded, clamped to
/// `0..=i16::MAX` — negative radii are not meaningful).
#[allow(
    clippy::cast_possible_truncation,
    reason = "value is clamped to 0..=i16::MAX before the cast — fits losslessly in i16"
)]
fn encode_px(value: f32) -> Px {
    let clamped = value.round().clamp(0.0, f32::from(i16::MAX));
    Px(clamped as i16)
}

/// Apply typography atoms into the text style sink. Typography atoms
/// carry their values directly (no theme resolution), so this is a free
/// function shared by [`StyleEngine::apply_text`] and
/// [`TextStyle::layered_over_binding`](crate::TextStyle::layered_over_binding).
pub(crate) fn apply_text_atoms(atoms: &[StyleAtom], text: &mut TextStyle) {
    for atom in atoms {
        apply_text_atom(*atom, text);
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive match on StyleAtom — the long no-op arm chain is intentional; a wildcard would silently swallow new atom kinds"
)]
fn apply_text_atom(atom: StyleAtom, text: &mut TextStyle) {
    match atom {
        StyleAtom::FontSize(px) => text.font_size = f32::from(px.0).max(1.0),
        StyleAtom::FontWeight(weight) => text.font_weight = weight.clamp(1, 1000),
        StyleAtom::FontFamily(name) => name.clone_into(&mut text.font_family),
        StyleAtom::LineHeight(px) => {
            text.line_height = TextLineHeight::Absolute(f32::from(px.0).max(1.0));
        }
        StyleAtom::PaddingAll(_)
        | StyleAtom::PaddingX(_)
        | StyleAtom::PaddingY(_)
        | StyleAtom::PaddingTop(_)
        | StyleAtom::PaddingRight(_)
        | StyleAtom::PaddingBottom(_)
        | StyleAtom::PaddingLeft(_)
        | StyleAtom::MarginAll(_)
        | StyleAtom::MarginX(_)
        | StyleAtom::MarginY(_)
        | StyleAtom::MarginTop(_)
        | StyleAtom::MarginRight(_)
        | StyleAtom::MarginBottom(_)
        | StyleAtom::MarginLeft(_)
        | StyleAtom::Gap(_)
        | StyleAtom::GapX(_)
        | StyleAtom::GapY(_)
        | StyleAtom::Width(_)
        | StyleAtom::Height(_)
        | StyleAtom::MinWidth(_)
        | StyleAtom::MaxWidth(_)
        | StyleAtom::MinHeight(_)
        | StyleAtom::MaxHeight(_)
        | StyleAtom::Display(_)
        | StyleAtom::FlexDirection(_)
        | StyleAtom::FlexWrap(_)
        | StyleAtom::AlignItems(_)
        | StyleAtom::JustifyContent(_)
        | StyleAtom::FlexGrow(_)
        | StyleAtom::FlexShrink(_)
        | StyleAtom::GridColumns(_)
        | StyleAtom::GridRows(_)
        | StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::BorderWidth(_)
        | StyleAtom::BorderColor(_)
        | StyleAtom::BorderThemeRef(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::Shadow(_)
        | StyleAtom::Opacity(_)
        | StyleAtom::OverflowX(_)
        | StyleAtom::OverflowY(_)
        | StyleAtom::Transition(_)
        | StyleAtom::TransitionDuration(_)
        | StyleAtom::UnknownThemeRef(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => {}
    }
}
