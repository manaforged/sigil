use crate::style::atom::{PaintAtom, StopSlot};
use crate::style::{
    AnimatorOverlay, Color, ColorAtomKind, ColorValue, NumericAtomKind, Px, RenderStyle, StyleAtom,
    TextLineHeight, TextStyle, Theme, Tone,
};

kittest_pub! {
    /// Applies style atoms to paint and text styles against a theme and tone.
    #[derive(Debug, Clone, Copy)]
    pub struct StyleEngine<'theme> {
        theme: &'theme Theme,
        tone: Tone,
    }
}

impl<'theme> StyleEngine<'theme> {
    kittest_pub! {
        /// Creates a style engine for `theme` with the neutral tone.
        #[must_use]
        pub const fn new(theme: &'theme Theme) -> Self {
            Self::with_tone(theme, Tone::Neutral)
        }
    }

    /// Creates an engine that resolves colors in `tone`.
    #[must_use]
    pub(crate) const fn with_tone(theme: &'theme Theme, tone: Tone) -> Self {
        Self { theme, tone }
    }

    kittest_pub! {
        /// Applies `atoms` to a render style.
        pub fn apply(&self, atoms: &[StyleAtom], render: &mut RenderStyle) {
            for atom in atoms {
                (*self).apply_atom(*atom, render);
            }
        }
    }

    pub(crate) fn overlay_render(overlay: &dyn AnimatorOverlay, render: &mut RenderStyle) {
        render.transform = render.transform.overlay(overlay);
        render.space.overlay(overlay);
        render.filter.overlay(overlay);
        render.paint.overlay(overlay);
        render.mask.overlay(overlay);
        if let Some(value) = overlay.value_for(NumericAtomKind::Opacity) {
            render.opacity = encode_opacity(value);
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::BorderRadius) {
            render.border_radius = Some(encode_px(value).0);
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

    /// Applies `atoms` to a text style.
    #[allow(clippy::unused_self, reason = "intentional, reviewed trade-off")]
    pub(crate) fn apply_text(&self, atoms: &[StyleAtom], text: &mut TextStyle) {
        apply_text_atoms(atoms, text);
    }

    #[must_use]
    fn tone_step(&self, step: u8) -> Color {
        self.theme.color(self.tone.step(step))
    }

    /// Resolves a color atom against the theme.
    #[must_use]
    pub(crate) fn resolve_color(&self, color: ColorValue) -> Color {
        self.theme.color(color)
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
            StyleAtom::TextColor(color) => render.text_color = Some(self.resolve_color(color)),
            StyleAtom::BgToneStep(step) => render.background_color = Some(self.tone_step(step)),
            StyleAtom::FgToneStep(step) => render.text_color = Some(self.tone_step(step)),
            StyleAtom::FgOnSolid => {
                render.text_color = Some(self.theme.scale(self.tone).on_solid);
            }
            StyleAtom::BorderRadius(radius) => render.border_radius = Some(radius),
            StyleAtom::Radius(radius) => {
                render.border_radius = Some(self.theme.metrics.radius_i16(radius));
            }
            StyleAtom::BorderWidth(width) => render.border_width = Some(width),
            StyleAtom::BorderTop(width) => render.border_top = Some(width),
            StyleAtom::BorderRight(width) => render.border_right = Some(width),
            StyleAtom::BorderBottom(width) => render.border_bottom = Some(width),
            StyleAtom::BorderLeft(width) => render.border_left = Some(width),
            StyleAtom::BorderColor(color) => render.border_color = Some(self.resolve_color(color)),
            StyleAtom::BorderToneStep(step) => render.border_color = Some(self.tone_step(step)),
            StyleAtom::Shadow(elevation) => render.shadow = Some(elevation),
            StyleAtom::Opacity(opacity) => render.opacity = opacity,
            #[cfg(feature = "kittest")]
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
            | StyleAtom::Width(_, _)
            | StyleAtom::Height(_, _)
            | StyleAtom::MinWidth(_, _)
            | StyleAtom::MaxWidth(_, _)
            | StyleAtom::MinHeight(_, _)
            | StyleAtom::MaxHeight(_, _)
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
            | StyleAtom::TransitionEase(_)
            | StyleAtom::TransitionDelay(_)
            | StyleAtom::Animation(_) => {}
            StyleAtom::Paint(paint) => self.apply_paint(paint, render),
            StyleAtom::ClipPath(shape) => render.clip_path = Some(shape),
            StyleAtom::Space(atom) => {
                render.transform.apply_space(atom);
                render.space.apply(atom);
            }
            StyleAtom::Mask(atom) => render.mask.apply(atom.0),
            StyleAtom::MixBlend(mode) => render.blend = mode,
            StyleAtom::Filter(atom) => render.filter.apply(atom.0),
            StyleAtom::DropShadowColor(color) => {
                render.filter.drop_color = Some(self.resolve_color(color));
            }
            StyleAtom::TextShadowColor(color) => {
                render.filter.text_color = Some(self.resolve_color(color));
            }
        }
    }

    fn apply_paint(self, atom: PaintAtom, render: &mut RenderStyle) {
        match atom {
            PaintAtom::Gradient(shape) => render.paint.gradient = Some(shape),
            PaintAtom::NoGradient => render.paint.gradient = None,
            PaintAtom::Stop(slot, color) => {
                set_stop(
                    &mut render.paint.stops,
                    slot,
                    Some(self.resolve_color(color)),
                );
            }
            PaintAtom::NoStop(slot) => set_stop(&mut render.paint.stops, slot, None),
            PaintAtom::StopAt(slot, percent) => {
                set_stop(&mut render.paint.positions, slot, Some(percent));
            }
            PaintAtom::NoColor => render.background_color = None,
            PaintAtom::ClipText => render.paint.clip_text = true,
            PaintAtom::Image(id) => {
                render.paint.image = Some(id);
                render.paint.shader = None;
            }
            PaintAtom::Shader(id) => {
                render.paint.shader = Some(id);
                render.paint.image = None;
            }
            PaintAtom::Border(style) => render.paint.border = style,
        }
    }
}

fn set_stop<T>(stops: &mut [Option<T>; 3], slot: StopSlot, value: Option<T>) {
    if let Some(stop) = stops.get_mut(slot.index()) {
        *stop = value;
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
pub(crate) fn encode_px(value: f32) -> Px {
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
        StyleAtom::FontSize(px) => text.font_size = f32::from(px).max(1.0),
        StyleAtom::FontWeight(weight) => text.font_weight = weight.clamp(1, 1000),
        StyleAtom::FontItalic(italic) => text.italic = italic,
        StyleAtom::FontFamily(name) => name.clone_into(&mut text.font_family),
        StyleAtom::LineHeight(px) => {
            text.line_height = TextLineHeight::Absolute(f32::from(px).max(1.0));
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
        | StyleAtom::Width(_, _)
        | StyleAtom::Height(_, _)
        | StyleAtom::MinWidth(_, _)
        | StyleAtom::MaxWidth(_, _)
        | StyleAtom::MinHeight(_, _)
        | StyleAtom::MaxHeight(_, _)
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
        | StyleAtom::TextColor(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::Radius(_)
        | StyleAtom::BorderWidth(_)
        | StyleAtom::BorderTop(_)
        | StyleAtom::BorderRight(_)
        | StyleAtom::BorderBottom(_)
        | StyleAtom::BorderLeft(_)
        | StyleAtom::BorderColor(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::Shadow(_)
        | StyleAtom::Opacity(_)
        | StyleAtom::OverflowX(_)
        | StyleAtom::OverflowY(_)
        | StyleAtom::Transition(_)
        | StyleAtom::TransitionDuration(_)
        | StyleAtom::TransitionEase(_)
        | StyleAtom::TransitionDelay(_)
        | StyleAtom::Animation(_)
        | StyleAtom::Paint(_)
        | StyleAtom::ClipPath(_)
        | StyleAtom::Space(_)
        | StyleAtom::Mask(_)
        | StyleAtom::MixBlend(_)
        | StyleAtom::Filter(_)
        | StyleAtom::DropShadowColor(_)
        | StyleAtom::TextShadowColor(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => {}
        #[cfg(feature = "kittest")]
        StyleAtom::Transform(_) => {}
    }
}
