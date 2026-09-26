use crate::style::scale::ScaleRole;
use crate::style::{
    AnimatorOverlay, ColorAtom, ColorAtomKind, NumericAtomKind, Px, RenderStyle, Rgba8, StyleAtom,
    TextLineHeight, TextStyle, Theme, ThemeValue, Tone,
};

#[derive(Debug, Clone, Copy)]
pub struct StyleEngine<'theme> {
    theme: &'theme Theme,
    tone: Tone,
}

impl<'theme> StyleEngine<'theme> {
    #[must_use]
    pub const fn new(theme: &'theme Theme) -> Self {
        Self::with_tone(theme, Tone::Neutral)
    }

    #[must_use]
    pub const fn with_tone(theme: &'theme Theme, tone: Tone) -> Self {
        Self { theme, tone }
    }

    pub fn apply(&self, atoms: &[StyleAtom], render: &mut RenderStyle) {
        for atom in atoms {
            (*self).apply_atom(*atom, render);
        }
    }

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

    pub(crate) fn overlay_render(overlay: &dyn AnimatorOverlay, render: &mut RenderStyle) {
        render.transform = render.transform.overlay(overlay);
        if let Some(value) = overlay.value_for(NumericAtomKind::Opacity) {
            render.opacity = encode_opacity(value);
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::BorderRadius) {
            render.border_radius = Some(encode_px(value));
        }
        if let Some(color) = overlay.value_for_color(ColorAtomKind::Background) {
            render.background_color = Some(color);
        }
        if let Some(color) = overlay.value_for_color(ColorAtomKind::Text) {
            render.text_color = Some(color);
        }
        if let Some(color) = overlay.value_for_color(ColorAtomKind::Border) {
            render.border_color = Some(color);
        }
    }

    pub(crate) fn overlay_text(overlay: &dyn AnimatorOverlay, text: &mut TextStyle) {
        if let Some(value) = overlay.value_for(NumericAtomKind::FontSize) {
            text.font_size = value.max(1.0);
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::LineHeight) {
            text.line_height = TextLineHeight::Absolute(value.max(1.0));
        }
    }

    #[allow(clippy::unused_self, reason = "intentional, reviewed trade-off")]
    pub fn apply_text(&self, atoms: &[StyleAtom], text: &mut TextStyle) {
        apply_text_atoms(atoms, text);
    }

    #[must_use]
    fn tone_step(&self, step: u8) -> Rgba8 {
        let role = self.tone.scale_role();
        let prefix = match role {
            ScaleRole::Gray => "gray",
            ScaleRole::Accent => "accent",
            ScaleRole::Success => "success",
            ScaleRole::Warn => "warn",
            ScaleRole::Error => "error",
            ScaleRole::Info => "info",
        };
        match self.theme.runtime_tokens.get(&format!("{prefix}-{step}")) {
            Some(ThemeValue::Color(color)) => *color,
            _ => self.theme.scale(role).step(step),
        }
    }

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
            StyleAtom::BackgroundGradient(gradient) => {
                render.background_gradient = Some(gradient);
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
            StyleAtom::Radius(radius) => {
                render.border_radius = Some(self.theme.metrics.radius(radius));
            }
            StyleAtom::BorderWidth(width) => render.border_width = Some(width),
            StyleAtom::BorderTop(width) => render.border_top = Some(width),
            StyleAtom::BorderRight(width) => render.border_right = Some(width),
            StyleAtom::BorderBottom(width) => render.border_bottom = Some(width),
            StyleAtom::BorderLeft(width) => render.border_left = Some(width),
            StyleAtom::BorderColor(color) => render.border_color = Some(self.resolve_color(color)),
            StyleAtom::BorderThemeRef(token) => {
                render.border_color = Some(self.theme.resolve_color(token));
            }
            StyleAtom::BorderToneStep(step) => render.border_color = Some(self.tone_step(step)),
            StyleAtom::Shadow(elevation) => render.shadow = Some(elevation),
            StyleAtom::Opacity(opacity) => render.opacity = opacity,
            StyleAtom::Transform(transform) => render.transform = transform,
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
            | StyleAtom::FontItalic(_)
            | StyleAtom::FontFamily(_)
            | StyleAtom::LineHeight(_)
            | StyleAtom::LetterSpacing(_)
            | StyleAtom::TabularNums(_)
            | StyleAtom::TextDecoration(_)
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
            StyleAtom::Radius(radius) => {
                let resolved = overlay
                    .value_for(NumericAtomKind::BorderRadius)
                    .map_or_else(|| self.theme.metrics.radius(radius), encode_px);
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

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is clamped to 0..=u16::MAX before the cast — fits losslessly in u16"
)]
fn encode_opacity(value: f32) -> u16 {
    value.clamp(0.0, f32::from(u16::MAX)).round() as u16
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "value is clamped to 0..=i16::MAX before the cast — fits losslessly in i16"
)]
fn encode_px(value: f32) -> Px {
    let clamped = value.round().clamp(0.0, f32::from(i16::MAX));
    Px(clamped as i16)
}

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
        StyleAtom::FontItalic(italic) => text.italic = italic,
        StyleAtom::FontFamily(name) => name.clone_into(&mut text.font_family),
        StyleAtom::LineHeight(px) => {
            text.line_height = TextLineHeight::Absolute(f32::from(px.0).max(1.0));
        }
        StyleAtom::TabularNums(on) => text.tabular_nums = on,
        StyleAtom::LetterSpacing(thousandths) => {
            text.letter_spacing_em = f32::from(thousandths) / 1000.0;
        }
        StyleAtom::TextDecoration(decoration) => text.decoration = decoration,
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
        | StyleAtom::BackgroundGradient(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::Radius(_)
        | StyleAtom::BorderWidth(_)
        | StyleAtom::BorderTop(_)
        | StyleAtom::BorderRight(_)
        | StyleAtom::BorderBottom(_)
        | StyleAtom::BorderLeft(_)
        | StyleAtom::BorderColor(_)
        | StyleAtom::BorderThemeRef(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::Shadow(_)
        | StyleAtom::Transform(_)
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
