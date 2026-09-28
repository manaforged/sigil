use super::mix::{alpha_mix, ramp_accent};
use super::{Theme, ThemeValue};
use crate::style::palette::is_dark;
use crate::style::{ColorKey, Rgba8, Token};

const SCRIM: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 153,
};

impl Theme {
    #[must_use]
    pub fn color(&self, token: Token) -> Rgba8 {
        self.resolve_color(ColorKey::Token(token))
    }

    #[must_use]
    pub fn resolve_color(&self, key: ColorKey) -> Rgba8 {
        self.tokens
            .overrides
            .color(key)
            .or_else(|| self.tokens.theme.color(key))
            .or_else(|| self.built_in_color(key))
            .unwrap_or_else(|| self.missing_color(key.name()))
    }

    pub fn register_token(&mut self, name: &str, value: ThemeValue) -> Option<ThemeValue> {
        if let (Some(token), ThemeValue::Color(color)) = (Token::from_name(name), value) {
            self.sync_runtime_color(token, color);
        }
        self.tokens.overrides.insert(name, value)
    }

    #[must_use]
    pub fn with_theme_tokens(mut self, tokens: &phf::Map<&'static str, ThemeValue>) -> Self {
        for (name, value) in tokens {
            self.tokens.theme.insert(name, *value);
        }
        self
    }

    #[must_use]
    pub fn with_token(mut self, name: &str, value: ThemeValue) -> Self {
        self.register_token(name, value);
        self
    }

    fn sync_runtime_color(&mut self, token: Token, color: Rgba8) {
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

    const fn field(&self) -> Rgba8 {
        if is_dark(self.surface) {
            self.gray.step(3)
        } else {
            self.gray.step(1)
        }
    }

    fn built_in_color(&self, key: ColorKey) -> Option<Rgba8> {
        let ColorKey::Token(token) = key else {
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
            Token::BorderStrong => self.border_strong(),
            Token::SurfaceSubtle => self.surface_subtle(),
            Token::SurfaceInset | Token::TableHeader => self.gray.step(1),
            Token::BorderSubtle => alpha_mix(self.surface, self.border, 0.5),
            Token::OnSurfaceMuted => self.text_muted(),
            Token::OnSolid => self.on_solid,
            Token::Scrim => SCRIM,
            _ => {
                let (role, step) = token.scale_step()?;
                self.scale(role).step(step)
            }
        })
    }

    pub(crate) fn legible(color: Rgba8, background: Rgba8) -> Rgba8 {
        let target = if luminance(background) < 0.18 {
            WHITE
        } else {
            BLACK
        };
        let mut out = color;
        for step in 1..=10_u8 {
            if contrast(out, background) >= LEGIBLE {
                return out;
            }
            out = alpha_mix(color, target, f32::from(step) / 10.0);
        }
        out
    }

    #[cfg(debug_assertions)]
    fn missing_color(&self, token: &str) -> Rgba8 {
        let message = self.unknown_token_message(token);
        diagnostic::unknown_theme_token(&message);
    }

    #[cfg(not(debug_assertions))]
    fn missing_color(&self, token: &'static str) -> Rgba8 {
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

const LEGIBLE: f32 = 4.5;
const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
const BLACK: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

fn luminance(color: Rgba8) -> f32 {
    let channel = |value: u8| {
        let c = f32::from(value) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.0722f32.mul_add(
        channel(color.b),
        0.2126f32.mul_add(channel(color.r), 0.7152 * channel(color.g)),
    )
}

fn contrast(a: Rgba8, b: Rgba8) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

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
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};

    static WARNED: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();

    #[cold]
    pub(super) fn warn_unknown_once(token: &'static str) {
        let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
        if warned.lock().is_ok_and(|mut set| set.insert(token)) {
            tracing::warn!(token, "unknown theme token");
        }
    }
}
