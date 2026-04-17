//! Style atom resolver.

use crate::{ColorAtom, RenderStyle, Rgba8, StyleAtom, Theme};

/// Resolves style atoms against a theme.
#[derive(Debug, Clone, Copy)]
pub struct StyleEngine<'theme> {
    theme: &'theme Theme,
}

impl<'theme> StyleEngine<'theme> {
    /// Create a resolver over a theme snapshot.
    #[must_use]
    pub const fn new(theme: &'theme Theme) -> Self {
        Self { theme }
    }

    /// Apply theme-backed visual atoms into the render sink.
    pub fn apply(&self, atoms: &[StyleAtom], render: &mut RenderStyle) {
        for atom in atoms {
            (*self).apply_atom(*atom, render);
        }
    }

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
            StyleAtom::BorderRadius(radius) => render.border_radius = Some(radius),
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
            | StyleAtom::Width(_)
            | StyleAtom::Height(_)
            | StyleAtom::MinWidth(_)
            | StyleAtom::MaxWidth(_)
            | StyleAtom::Display(_)
            | StyleAtom::FlexDirection(_)
            | StyleAtom::AlignItems(_)
            | StyleAtom::JustifyContent(_)
            | StyleAtom::FlexGrow(_)
            | StyleAtom::FlexShrink(_)
            | StyleAtom::FontSize(_)
            | StyleAtom::FontWeight(_)
            | StyleAtom::LineHeight(_)
            | StyleAtom::Transition(_)
            | StyleAtom::TransitionDuration(_)
            | StyleAtom::UnknownThemeRef(_) => {}
        }
    }

    fn resolve_color(self, color: ColorAtom) -> Rgba8 {
        match color {
            ColorAtom::Rgba8(color) => color,
            ColorAtom::ThemeRef(token) => self.theme.resolve_color(token),
        }
    }
}
