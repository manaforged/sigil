//! Theme tokens and palette values.

use crate::Rgba8;

#[cfg(debug_assertions)]
const BUILT_IN_COLOR_TOKENS: [&str; 7] = [
    "accent",
    "accent-hover",
    "on-accent",
    "surface",
    "surface-hover",
    "on-surface",
    "border",
];

/// A value stored in the user-extensible theme token map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeValue {
    /// A concrete colour.
    Color(Rgba8),
    /// A logical-pixel value.
    Px(f32),
    /// A millisecond duration.
    Ms(u16),
}

/// Runtime theme snapshot used to resolve style atoms.
#[derive(Debug, PartialEq)]
pub struct Theme {
    /// Primary action/accent colour.
    pub accent: Rgba8,
    /// Hover state for primary action/accent surfaces.
    pub accent_hover: Rgba8,
    /// Text/icon colour on accent surfaces.
    pub on_accent: Rgba8,
    /// Default panel/surface colour.
    pub surface: Rgba8,
    /// Hover state for default surfaces.
    pub surface_hover: Rgba8,
    /// Text/icon colour on default surfaces.
    pub on_surface: Rgba8,
    /// Default border colour.
    pub border: Rgba8,
    /// Release-mode fallback for missing theme tokens.
    pub missing_token_fallback: Rgba8,
    /// User-defined tokens.
    pub tokens: phf::Map<&'static str, ThemeValue>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: rgba(51, 112, 255), // Clear accent for proof apps and early widgets.
            accent_hover: rgba(35, 93, 230), // Slightly deeper accent for hover feedback.
            on_accent: rgba(255, 255, 255), // White keeps accent controls readable.
            surface: rgba(248, 250, 252), // Near-white default pane surface.
            surface_hover: rgba(226, 232, 240), // Cool hover lift for neutral surfaces.
            on_surface: rgba(15, 23, 42), // Dark text for light surfaces.
            border: rgba(148, 163, 184), // Subtle cool border.
            missing_token_fallback: rgba(255, 0, 255), // Debug magenta for misses.
            tokens: phf::Map::new(),
        }
    }
}

impl Theme {
    pub(crate) fn resolve_color(&self, token: &'static str) -> Rgba8 {
        if let Some(color) = self.built_in_color(token) {
            return color;
        }
        if let Some(ThemeValue::Color(color)) = self.tokens.get(token).copied() {
            return color;
        }
        self.missing_color(token)
    }

    fn built_in_color(&self, token: &str) -> Option<Rgba8> {
        match token {
            "accent" => Some(self.accent),
            "accent-hover" => Some(self.accent_hover),
            "on-accent" => Some(self.on_accent),
            "surface" => Some(self.surface),
            "surface-hover" => Some(self.surface_hover),
            "on-surface" => Some(self.on_surface),
            "border" => Some(self.border),
            _ => None,
        }
    }

    #[cfg(debug_assertions)]
    fn missing_color(&self, token: &str) -> Rgba8 {
        let message = self.unknown_token_message(token);
        diagnostic::unknown_theme_token(&message);
    }

    #[cfg(not(debug_assertions))]
    const fn missing_color(&self, _token: &str) -> Rgba8 {
        self.missing_token_fallback
    }

    #[cfg(debug_assertions)]
    fn unknown_token_message(&self, token: &str) -> String {
        let registered = self.registered_tokens();
        let nearest = nearest_tokens(token, &registered);
        format!(
            "Unknown theme token '{token}'; registered: {}; nearest: '{}', '{}', '{}'",
            registered.join(", "),
            nearest[0],
            nearest[1],
            nearest[2],
        )
    }

    #[cfg(debug_assertions)]
    fn registered_tokens(&self) -> Vec<&'static str> {
        let mut tokens = BUILT_IN_COLOR_TOKENS.to_vec();
        tokens.extend(self.tokens.keys().copied());
        tokens
    }
}

/// Builder for constructing a theme with user tokens.
#[derive(Debug, Default, PartialEq)]
pub struct ThemeBuilder {
    theme: Theme,
}

impl ThemeBuilder {
    /// Start from the default Sigil theme.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the user token map.
    #[must_use]
    pub const fn tokens(mut self, tokens: phf::Map<&'static str, ThemeValue>) -> Self {
        self.theme.tokens = tokens;
        self
    }

    /// Build the theme.
    #[must_use]
    pub const fn build(self) -> Theme {
        self.theme
    }
}

#[cfg(debug_assertions)]
fn nearest_tokens(token: &str, registered: &[&'static str]) -> [&'static str; 3] {
    let mut scored = registered
        .iter()
        .copied()
        .map(|candidate| (strsim::levenshtein(token, candidate), candidate))
        .collect::<Vec<_>>();
    scored.sort_by_key(|(score, candidate)| (*score, *candidate));
    [
        scored.first().map_or("", |(_, candidate)| *candidate),
        scored.get(1).map_or("", |(_, candidate)| *candidate),
        scored.get(2).map_or("", |(_, candidate)| *candidate),
    ]
}

const fn rgba(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

#[cfg(debug_assertions)]
mod diagnostic {
    #![allow(clippy::panic)]

    #[cold]
    #[track_caller]
    pub(super) fn unknown_theme_token(message: &str) -> ! {
        panic!("{message}");
    }
}
