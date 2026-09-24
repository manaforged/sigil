use super::mix::{alpha_mix, ramp_accent};
use super::{Theme, ThemeValue};
use crate::style::palette::is_dark;
use crate::style::scale::ScaleRole;
use crate::style::{Rgba8, Token};

const SCRIM: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 153,
};

impl Theme {
    #[must_use]
    pub fn color(&self, token: Token) -> Rgba8 {
        self.resolve_color(token.name())
    }

    pub(crate) fn resolve_color(&self, token: &'static str) -> Rgba8 {
        if let Some(ThemeValue::Color(color)) = self.runtime_tokens.get(token).copied() {
            return color;
        }
        if let Some(ThemeValue::Color(color)) = self.tokens.get(token).copied() {
            return color;
        }
        if let Some(color) = self.built_in_color(token) {
            return color;
        }
        if let Some(color) = self.scale_step_color(token) {
            return color;
        }
        self.missing_color(token)
    }

    pub fn register_token(
        &mut self,
        name: impl Into<String>,
        value: ThemeValue,
    ) -> Option<ThemeValue> {
        let name = name.into();
        if let ThemeValue::Color(color) = value {
            self.sync_runtime_color(&name, color);
        }
        self.runtime_tokens.insert(name, value)
    }

    #[must_use]
    pub fn with_token(mut self, name: impl Into<String>, value: ThemeValue) -> Self {
        self.register_token(name, value);
        self
    }

    fn sync_runtime_color(&mut self, name: &str, color: Rgba8) {
        match name {
            "accent" => {
                self.accent = color;
                self.accent_scale = ramp_accent(self.surface, color);
            }
            "accent-hover" => {
                self.accent_hover = color;
                self.accent_scale.steps[9] = color;
            }
            "on-accent" => {
                self.on_accent = color;
                self.accent_scale.on_solid = color;
            }
            "surface" => self.surface = color,
            "surface-hover" => self.surface_hover = color,
            "surface-pressed" => self.surface_pressed = color,
            "surface-pressed-strong" => self.surface_pressed_strong = color,
            "on-surface" => self.on_surface = color,
            "border" => self.border = color,
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

    fn built_in_color(&self, token: &str) -> Option<Rgba8> {
        match token {
            "accent" | "ring" => Some(self.accent),
            "accent-hover" => Some(self.accent_hover),
            "on-accent" => Some(self.on_accent),
            "field" | "overlay" => Some(self.field()),
            "surface" => Some(self.surface),
            "surface-hover" => Some(self.surface_hover),
            "surface-pressed" => Some(self.surface_pressed),
            "surface-pressed-strong" => Some(self.surface_pressed_strong),
            "on-surface" => Some(self.on_surface),
            "border" | "overlay-border" => Some(self.border),
            "border-strong" => Some(self.border_strong()),
            "surface-subtle" => Some(self.surface_subtle()),
            "surface-inset" => Some(self.gray.step(1)),
            "on-surface-muted" => Some(self.text_muted()),
            "on-solid" => Some(self.on_solid),
            "scrim" => Some(SCRIM),
            _ => None,
        }
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

    fn scale_step_color(&self, token: &str) -> Option<Rgba8> {
        let (role_name, step_str) = token.rsplit_once('-')?;
        let role = ScaleRole::from_token(role_name)?;
        let step: u8 = step_str.parse().ok()?;
        if !(1..=12).contains(&step) {
            return None;
        }
        Some(self.scale(role).step(step))
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
    fn registered_tokens(&self) -> Vec<String> {
        let mut tokens: Vec<String> = Token::ALL.iter().map(|t| t.name().into()).collect();
        tokens.extend(self.tokens.keys().map(|s| (*s).into()));
        tokens.extend(self.runtime_tokens.keys().cloned());
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
