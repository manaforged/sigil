use super::mix::{alpha_mix, ramp_accent};
use super::{Theme, ThemeValue};
use crate::style::palette::is_dark;
use crate::style::srgb;
use crate::style::{Color, ColorValue, Token};

impl From<Color> for ThemeValue {
    fn from(color: Color) -> Self {
        Self::Color(color)
    }
}

const SCRIM: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 153,
};

impl Theme {
    /// Returns the color a token, custom token name, or literal color resolves to in this theme.
    #[must_use]
    pub fn color(&self, color: impl Into<ColorValue>) -> Color {
        let color = color.into();
        let name = match color {
            ColorValue::Color(rgba) => return rgba,
            ColorValue::Token(token) => token.name(),
            ColorValue::Custom(name) => name,
        };
        self.tokens
            .overrides
            .color(color)
            .or_else(|| self.tokens.theme.color(color))
            .or_else(|| self.built_in_color(color))
            .unwrap_or_else(|| self.missing_color(name))
    }

    /// Sets the token `name`, built-in or custom, to `value`.
    #[must_use]
    pub fn token(mut self, name: &str, value: impl Into<ThemeValue>) -> Self {
        let value = value.into();
        if let (Some(token), ThemeValue::Color(color)) = (Token::from_name(name), value) {
            self.sync_runtime_color(token, color);
        }
        self.tokens.overrides.insert(name, value);
        self
    }

    /// Fill the theme table layer from static data.
    #[must_use]
    pub(crate) fn theme_tokens(mut self, tokens: &phf::Map<&'static str, ThemeValue>) -> Self {
        for (name, value) in tokens {
            self.tokens.theme.insert(name, *value);
        }
        self
    }

    fn sync_runtime_color(&mut self, token: Token, color: Color) {
        match token {
            Token::Accent => {
                self.accent = color;
                self.accent_scale = ramp_accent(self.surface, color);
            }
            Token::AccentHover => {
                self.accent_hover = color;
                self.accent_scale.steps[9] = color;
            }
            Token::OnAccent => {
                self.on_accent = color;
                self.accent_scale.on_solid = color;
            }
            Token::Surface => self.surface = color,
            Token::SurfaceHover => self.surface_hover = color,
            Token::SurfacePressed => self.surface_pressed = color,
            Token::SurfacePressedStrong => self.surface_pressed_strong = color,
            Token::OnSurface => self.on_surface = color,
            Token::Border => self.border = color,
            _ => {}
        }
    }

    const fn field(&self) -> Color {
        if is_dark(self.surface) {
            self.gray.step(3)
        } else {
            self.gray.step(1)
        }
    }

    fn built_in_color(&self, key: ColorValue) -> Option<Color> {
        let ColorValue::Token(token) = key else {
            return None;
        };
        Some(match token {
            Token::Accent | Token::Ring => self.accent,
            Token::AccentHover => self.accent_hover,
            Token::OnAccent => self.on_accent,
            Token::Field | Token::Overlay => self.field(),
            Token::Surface => self.surface,
            Token::SurfaceHover => self.surface_hover,
            Token::SurfacePressed => self.surface_pressed,
            Token::SurfacePressedStrong => self.surface_pressed_strong,
            Token::OnSurface => self.on_surface,
            Token::Border | Token::OverlayBorder => self.border,
            Token::BorderStrong => self.gray.step(8),
            Token::SurfaceSubtle => self.gray.step(3),
            Token::SurfaceInset | Token::TableHeader => self.gray.step(1),
            Token::BorderSubtle => alpha_mix(self.surface, self.border, 0.5),
            Token::OnSurfaceMuted | Token::Placeholder => self.gray.step(11),
            Token::OnSolid => self.on_solid,
            Token::Scrim => SCRIM,
            _ => {
                let (role, step) = token.scale_step()?;
                self.scale(role).step(step)
            }
        })
    }

    pub(crate) fn legible(color: Color, background: Color) -> Color {
        let target = if srgb::luminance(background) < 0.18 {
            WHITE
        } else {
            BLACK
        };
        let mut out = color;
        for step in 1..=10_u8 {
            if srgb::contrast(out, background) >= srgb::AA_TEXT {
                return out;
            }
            out = alpha_mix(color, target, f32::from(step) / 10.0);
        }
        out
    }

    #[cfg(debug_assertions)]
    fn missing_color(&self, token: &str) -> Color {
        let message = self.unknown_token_message(token);
        diagnostic::unknown_theme_token(&message);
    }

    #[cfg(not(debug_assertions))]
    fn missing_color(&self, token: &'static str) -> Color {
        diagnostic::warn_unknown_once(token);
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
    fn registered_tokens(&self) -> Vec<String> {
        let mut tokens: Vec<String> = Token::ALL.iter().map(|t| t.name().into()).collect();
        tokens.extend(self.tokens.theme.names().map(String::from));
        tokens.extend(self.tokens.overrides.names().map(String::from));
        tokens
    }
}

const WHITE: Color = Color::rgb(255, 255, 255);
const BLACK: Color = Color::rgb(0, 0, 0);

#[cfg(debug_assertions)]
fn nearest_tokens(token: &str, registered: &[String]) -> [String; 3] {
    let mut scored: Vec<(usize, &String)> = registered
        .iter()
        .map(|candidate| (strsim::levenshtein(token, candidate), candidate))
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let pick = |i: usize| {
        scored
            .get(i)
            .map(|(_, candidate)| (*candidate).clone())
            .unwrap_or_default()
    };
    [pick(0), pick(1), pick(2)]
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

#[cfg(not(debug_assertions))]
mod diagnostic {
    use crate::style::diagnostic::Seen;

    static WARNED: Seen<&'static str> = Seen::new();

    #[cold]
    pub(super) fn warn_unknown_once(token: &'static str) {
        if WARNED.first(token) {
            tracing::warn!(token, "unknown theme token");
        }
    }
}
