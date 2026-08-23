//! Theme tokens and palette values.

use std::collections::HashMap;

use crate::palette::Palette;
use crate::scale::{
    ColorScale, ScaleRole, amber_dark, amber_light, blue_dark, blue_light, cyan_dark, cyan_light,
    green_dark, green_light, ruby_dark, ruby_light, slate_dark, slate_light,
};
use crate::variant::{FontFamilyHint, Variant};
use crate::{Dimension, LengthPercentage, Px, Rgba8, Size};

#[cfg(debug_assertions)]
const BUILT_IN_COLOR_TOKENS: [&str; 17] = [
    "accent",
    "accent-hover",
    "on-accent",
    "surface",
    "surface-hover",
    "surface-pressed",
    "surface-pressed-strong",
    "surface-inset",
    "on-surface",
    "border",
    "border-strong",
    "scrim",
    "surface-0",
    "surface-1",
    "surface-2",
    "surface-3",
    "line",
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
///
/// As of DS1 the theme carries one 12-step [`ColorScale`] per role
/// (gray / accent / success / warn / error / info). The flat fields
/// (`accent`, `surface`, …) are kept as a backwards-compat shim and
/// are populated from the corresponding scale steps; new code should
/// reach for the scale or for the semantic alias helpers
/// ([`Theme::surface`], `Theme::text_default`, etc.) instead.
#[derive(Debug, PartialEq)]
pub struct Theme {
    /// Neutral gray scale — surfaces, borders, default text.
    pub gray: ColorScale,
    /// Accent / brand scale — primary actions, focus rings.
    pub accent_scale: ColorScale,
    /// Success scale — confirmation chips, valid borders.
    pub success: ColorScale,
    /// Warn scale — cautionary banners, non-blocking notices.
    pub warn: ColorScale,
    /// Error / destructive scale — invalid inputs, delete actions.
    pub error: ColorScale,
    /// Info / notice scale — hint banners, neutral callouts.
    pub info: ColorScale,
    /// Text/icon colour rendered on top of a solid (step-9) fill —
    /// usually white, contrasted against accent / success / error
    /// solid backgrounds. Independent of the gray scale because the
    /// solid step lives outside the neutral range.
    pub on_solid: Rgba8,

    /// Primary action/accent colour. Mirrors `accent_scale.step(9)`.
    pub accent: Rgba8,
    /// Hover state for primary action/accent surfaces. Mirrors
    /// `accent_scale.step(10)`.
    pub accent_hover: Rgba8,
    /// Text/icon colour on accent surfaces. Mirrors [`Self::on_solid`].
    pub on_accent: Rgba8,
    /// Default panel/surface colour. Mirrors `gray.step(2)`.
    pub surface: Rgba8,
    /// Hover state for default surfaces. Mirrors `gray.step(4)`.
    pub surface_hover: Rgba8,
    /// Pressed/active state for default surfaces. Mirrors
    /// `gray.step(5)`.
    pub surface_pressed: Rgba8,
    /// Deepest pressed-state tint for tiny indicators (16-20 px).
    /// Mirrors `gray.step(6)`.
    pub surface_pressed_strong: Rgba8,
    /// Text/icon colour on default surfaces. Mirrors `gray.step(12)`.
    pub on_surface: Rgba8,
    /// Default border colour. Mirrors `gray.step(7)`.
    pub border: Rgba8,
    /// Release-mode fallback for missing theme tokens.
    pub missing_token_fallback: Rgba8,
    /// Compile-time perfect-hash user tokens (built via `phf_map!`).
    pub tokens: phf::Map<&'static str, ThemeValue>,
    /// Runtime-registered user tokens — overlays `tokens`.
    ///
    /// Use [`Theme::with_token`] / [`Theme::register_token`] to add an
    /// entry. Runtime tokens shadow `phf` tokens with the same key, so
    /// callers can override compile-time defaults at startup.
    pub runtime_tokens: HashMap<String, ThemeValue>,
}

impl Theme {
    /// Borrow the scale assigned to the given semantic [`ScaleRole`].
    #[must_use]
    pub const fn scale(&self, role: ScaleRole) -> &ColorScale {
        match role {
            ScaleRole::Gray => &self.gray,
            ScaleRole::Accent => &self.accent_scale,
            ScaleRole::Success => &self.success,
            ScaleRole::Warn => &self.warn,
            ScaleRole::Error => &self.error,
            ScaleRole::Info => &self.info,
        }
    }

    /// Replace the gray scale and rebuild the flat-field shim. Use
    /// this to swap the neutral hue (e.g. mauve / sage / olive) for
    /// the entire theme.
    #[must_use]
    pub const fn with_gray_scale(mut self, scale: ColorScale) -> Self {
        self.gray = scale;
        self.surface = scale.step(2);
        self.surface_hover = scale.step(4);
        self.surface_pressed = scale.step(5);
        self.surface_pressed_strong = scale.step(6);
        self.border = scale.step(7);
        self.on_surface = scale.step(12);
        self
    }

    /// Replace the accent scale and rebuild the flat-field shim.
    #[must_use]
    pub const fn with_accent_scale(mut self, scale: ColorScale) -> Self {
        self.accent_scale = scale;
        self.accent = scale.step(9);
        self.accent_hover = scale.step(10);
        self
    }

    /// Replace the success scale.
    #[must_use]
    pub const fn with_success_scale(mut self, scale: ColorScale) -> Self {
        self.success = scale;
        self
    }

    /// Replace the warn scale.
    #[must_use]
    pub const fn with_warn_scale(mut self, scale: ColorScale) -> Self {
        self.warn = scale;
        self
    }

    /// Replace the error scale.
    #[must_use]
    pub const fn with_error_scale(mut self, scale: ColorScale) -> Self {
        self.error = scale;
        self
    }

    /// Replace the info scale.
    #[must_use]
    pub const fn with_info_scale(mut self, scale: ColorScale) -> Self {
        self.info = scale;
        self
    }

    /// Semantic alias: low-contrast / muted text on default surfaces.
    /// Reads `gray.step(11)`.
    #[must_use]
    pub const fn text_muted(&self) -> Rgba8 {
        self.gray.step(11)
    }

    /// Semantic alias: subtle background — slightly darker than the
    /// app shell, used for cards / panels. Reads `gray.step(3)`.
    #[must_use]
    pub const fn surface_subtle(&self) -> Rgba8 {
        self.gray.step(3)
    }

    /// Semantic alias: stronger border for focused / hovered shells.
    /// Reads `gray.step(8)`.
    #[must_use]
    pub const fn border_strong(&self) -> Rgba8 {
        self.gray.step(8)
    }
}

impl Theme {
    /// Build a dark-mode theme.
    ///
    /// Mirrors [`Theme::default`] (the light-mode constructor) but
    /// uses the dark variant of every scale. Semantic aliases keep
    /// their step references — `surface` is still `gray.step(2)`,
    /// just `slate_dark().step(2)` instead of `slate_light().step(2)`.
    /// Widgets that go through `bg-surface` / `text-on-surface` /
    /// `bg-success-3` / etc. flip automatically; widgets that
    /// hard-code RGB values do not (audit them with a `grep
    /// "Rgba8 {"` sweep before promoting to DS-compliant).
    #[must_use]
    pub fn dark() -> Self {
        let gray = slate_dark();
        let accent_scale = blue_dark();
        let success = green_dark();
        let warn = amber_dark();
        let error = ruby_dark();
        let info = cyan_dark();
        let on_solid = rgba(255, 255, 255);
        Self {
            gray,
            accent_scale,
            success,
            warn,
            error,
            info,
            on_solid,
            accent: accent_scale.step(9),
            accent_hover: accent_scale.step(10),
            on_accent: on_solid,
            surface: gray.step(2),
            surface_hover: gray.step(4),
            surface_pressed: gray.step(5),
            surface_pressed_strong: gray.step(6),
            on_surface: gray.step(12),
            border: gray.step(7),
            missing_token_fallback: rgba(255, 0, 255),
            tokens: phf::Map::new(),
            runtime_tokens: crate::grid::tokens(),
        }
    }

    /// Build the the host app parity theme: Sigil's dark scales plus the shadcn
    /// (new-york, dark) token set bound to the exact sRGB the React app renders.
    ///
    /// Values are captured from the running the host app app, not
    /// hand-computed.
    /// Because these tokens are registered explicitly, `resolve_color` lets them win over
    /// the built-in `accent` / `border`, so a ported React `className` keeps its token
    /// words verbatim: `bg-card`, `text-muted-foreground`, `bg-destructive`. `border` and
    /// `input` carry their shadcn alpha (white at 10% / 15%).
    #[must_use]
    // Not `const`: DS2.5's `runtime_tokens` HashMap made `dark()`
    // non-const, and this preset builds on it via struct update.
    pub fn console_dark() -> Self {
        Self {
            tokens: phf::phf_map! {
                "background" => ThemeValue::Color(Rgba8 { r: 10, g: 10, b: 10, a: 255 }),
                "foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "card" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "card-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "popover" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "popover-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "primary" => ThemeValue::Color(Rgba8 { r: 229, g: 229, b: 229, a: 255 }),
                "primary-foreground" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "secondary" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "secondary-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "muted" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "muted-foreground" => ThemeValue::Color(Rgba8 { r: 161, g: 161, b: 161, a: 255 }),
                "accent" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "accent-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "destructive" => ThemeValue::Color(Rgba8 { r: 255, g: 100, b: 103, a: 255 }),
                // shadcn pairs `bg-destructive` with `text-destructive-foreground`
                // (e.g. the chat Stop chip). The React extract only captured the
                // `destructive` bg; its foreground is the same near-white the rest
                // of this dark theme uses for on-color text, so the label stays
                // readable on the red. Without this, resolving the class panics
                // (debug) / renders magenta (release).
                "destructive-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "border" => ThemeValue::Color(Rgba8 { r: 245, g: 255, b: 255, a: 26 }),
                "input" => ThemeValue::Color(Rgba8 { r: 255, g: 255, b: 255, a: 38 }),
                "ring" => ThemeValue::Color(Rgba8 { r: 115, g: 115, b: 115, a: 255 }),
                "sidebar" => ThemeValue::Color(Rgba8 { r: 23, g: 23, b: 23, a: 255 }),
                "sidebar-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-primary" => ThemeValue::Color(Rgba8 { r: 20, g: 71, b: 230, a: 255 }),
                "sidebar-primary-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-accent" => ThemeValue::Color(Rgba8 { r: 38, g: 38, b: 38, a: 255 }),
                "sidebar-accent-foreground" => ThemeValue::Color(Rgba8 { r: 250, g: 250, b: 250, a: 255 }),
                "sidebar-border" => ThemeValue::Color(Rgba8 { r: 245, g: 255, b: 255, a: 26 }),
                "sidebar-ring" => ThemeValue::Color(Rgba8 { r: 115, g: 115, b: 115, a: 255 }),
                // Syntax-highlight palette (One Dark-ish), consumed by `code_edit` via
                // per-run `text_theme("syntax-*")`. Capture kinds map to these in the
                // editor consumer; an unmapped kind falls back to `foreground`.
                "syntax-keyword" => ThemeValue::Color(Rgba8 { r: 198, g: 120, b: 221, a: 255 }),
                "syntax-string" => ThemeValue::Color(Rgba8 { r: 152, g: 195, b: 121, a: 255 }),
                "syntax-comment" => ThemeValue::Color(Rgba8 { r: 117, g: 123, b: 138, a: 255 }),
                "syntax-function" => ThemeValue::Color(Rgba8 { r: 97, g: 175, b: 239, a: 255 }),
                "syntax-type" => ThemeValue::Color(Rgba8 { r: 229, g: 192, b: 123, a: 255 }),
                "syntax-number" => ThemeValue::Color(Rgba8 { r: 209, g: 154, b: 102, a: 255 }),
                "syntax-constant" => ThemeValue::Color(Rgba8 { r: 209, g: 154, b: 102, a: 255 }),
                "syntax-property" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
                "syntax-operator" => ThemeValue::Color(Rgba8 { r: 86, g: 182, b: 194, a: 255 }),
                "syntax-punctuation" => ThemeValue::Color(Rgba8 { r: 171, g: 178, b: 191, a: 255 }),
                "syntax-variable" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
                "syntax-tag" => ThemeValue::Color(Rgba8 { r: 224, g: 108, b: 117, a: 255 }),
            },
            ..Self::dark()
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        // Default scales — Radix Colors-inspired light theme. The flat
        // fields below are derived from these so legacy code that
        // reaches for `theme.surface` / `theme.accent` keeps working
        // verbatim.
        let gray = slate_light();
        let accent_scale = blue_light();
        let success = green_light();
        let warn = amber_light();
        let error = ruby_light();
        let info = cyan_light();
        let on_solid = rgba(255, 255, 255);
        Self {
            gray,
            accent_scale,
            success,
            warn,
            error,
            info,
            on_solid,
            accent: accent_scale.step(9),
            accent_hover: accent_scale.step(10),
            on_accent: on_solid,
            surface: gray.step(2),
            surface_hover: gray.step(4),
            surface_pressed: gray.step(5),
            surface_pressed_strong: gray.step(6),
            on_surface: gray.step(12),
            border: gray.step(7),
            missing_token_fallback: rgba(255, 0, 255), // Debug magenta for misses.
            tokens: phf::Map::new(),
            runtime_tokens: crate::grid::tokens(),
        }
    }
}

impl Theme {
    pub(crate) fn resolve_color(&self, token: &'static str) -> Rgba8 {
        // Runtime tokens (theme.toml / register_token) win, then compile-time
        // `tokens`, then built-in / scale names. A theme file can rebind
        // `accent` / `border` / `surface` the same way `console_dark` does.
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

    /// Register a runtime token. Overlays a same-named compile-time
    /// `phf` token and any built-in of the same name. Returns the
    /// previously-bound value, if any.
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

    /// Builder-style alternative to [`Theme::register_token`].
    #[must_use]
    pub fn with_token(mut self, name: impl Into<String>, value: ThemeValue) -> Self {
        self.register_token(name, value);
        self
    }

    /// Keep flat fields and the accent scale in lockstep with a
    /// same-named runtime color so `Tone::Primary` and `theme.accent`
    /// follow theme.toml.
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

    fn built_in_color(&self, token: &str) -> Option<Rgba8> {
        match token {
            // Legacy named tokens.
            "accent" => Some(self.accent),
            "accent-hover" => Some(self.accent_hover),
            "on-accent" => Some(self.on_accent),
            "surface" | "surface-1" => Some(self.surface),
            "surface-hover" => Some(self.surface_hover),
            "surface-pressed" | "surface-2" => Some(self.surface_pressed),
            "surface-pressed-strong" | "surface-3" => Some(self.surface_pressed_strong),
            "on-surface" => Some(self.on_surface),
            "border" | "line" => Some(self.border),
            // Step 8 — quiet-but-measurable track/boundary color for
            // linear meters (slider rest, gauge, progress).
            "border-strong" => Some(self.border_strong()),
            // DS1 semantic aliases. The parser strips the `bg-` /
            // `text-` prefix before lookup, so utility classes like
            // `bg-surface-subtle` / `text-on-surface-muted` route here
            // by their stripped suffix.
            "surface-subtle" => Some(self.surface_subtle()),
            // Inset well / page canvas (surface-0). Step 1 reads darker
            // than the card in dark themes and lighter in light themes.
            "surface-inset" | "surface-0" => Some(self.gray.step(1)),
            "on-surface-muted" => Some(self.text_muted()),
            "on-solid" => Some(self.on_solid),
            // Modal-style overlay scrim — 60 % opacity black
            // (alpha 153 ≈ 0.60 * 255). Themable so a preset can
            // rebind it via `tokens` / `runtime_tokens` like any
            // other color token.
            "scrim" => Some(Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 153,
            }),
            _ => None,
        }
    }

    /// Resolve a `<role>-<step>` token such as `gray-2`, `success-9`,
    /// or `error-11` to its concrete color. Returns `None` when the
    /// token is not in scale form so the caller can fall through to
    /// user-defined tokens / the unknown-token fallback.
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
        let mut tokens: Vec<String> = BUILT_IN_COLOR_TOKENS.iter().map(|s| (*s).into()).collect();
        tokens.extend(self.tokens.keys().map(|s| (*s).into()));
        tokens.extend(self.runtime_tokens.keys().cloned());
        tokens
    }

    /// Compose a [`Theme`] from a six-role [`Palette`] and a shape
    /// [`Variant`]. The internal seven-token colour set (accent /
    /// surface / border family) is derived from the palette; the
    /// variant's hover behaviour decides whether `surface_hover` and
    /// `accent_hover` shift versus their base.
    ///
    /// This is the foundational constructor every named theme
    /// (Obsidian, Mythril, Aetherlight, …) delegates to.
    #[must_use]
    pub fn from_palette(palette: Palette, variant: Variant) -> Self {
        let tokens = variant.tokens();
        let dark_surface = is_dark(palette.background);
        // Hover shifts by ~26 lightness units (was 18). The 18-unit
        // value gave only ~8 sRGB delta after gamma encoding —
        // borderline-to-below the same-hue JND (~7.65) and sub-
        // perceptual on a 16-22px widget swatch. 26 lifts the post-
        // gamma delta clear of the JND across calibration variance.
        // Pressed doubles this (saturating to 52) so the hover→pressed
        // step is itself perceptible.
        let hover_shift: i16 = if tokens.hover_lift {
            if dark_surface { 26 } else { -26 }
        } else {
            // Variant suppresses hover lift (e.g. Terminal): surfaces
            // stay flat regardless of pointer state.
            0
        };
        // Text-on-accent: pick whichever of the palette's foreground or
        // background colour better contrasts with the accent. The prior
        // `on_accent: palette.background` was correct only by accident
        // for palettes where the accent and the background sat on
        // opposite ends of the brightness axis; for `Spellforge` with a
        // dark accent on a dark palette it produced unreadable dark-on-
        // dark text.
        let on_accent = pick_higher_contrast(palette.primary, palette.text, palette.background);
        // The DS1 scales are not yet derivable from a 6-color Palette;
        // fall back to the default light scales so role-prefixed
        // classes (`bg-success-3`, etc.) still resolve. A future
        // palette-aware pipeline can replace these by interpolating
        // around each palette role color.
        Self {
            gray: slate_light(),
            accent_scale: blue_light(),
            success: green_light(),
            warn: amber_light(),
            error: ruby_light(),
            info: cyan_light(),
            on_solid: rgba(255, 255, 255),
            accent: palette.primary,
            accent_hover: shift(palette.primary, hover_shift),
            on_accent,
            surface: palette.background,
            surface_hover: shift(palette.background, hover_shift),
            // Pressed shifts twice as far as hover — a Terminal-flat
            // variant collapses both to the base alongside hover.
            surface_pressed: shift(palette.background, hover_shift.saturating_mul(2)),
            // Strong-pressed shifts a further `hover_shift` past pressed
            // (3× hover from surface). Terminal-flat variants collapse
            // alongside hover/pressed. Targeted at 16-20 px indicators.
            surface_pressed_strong: shift(palette.background, hover_shift.saturating_mul(3)),
            on_surface: palette.text,
            border: alpha_mix(palette.text, palette.background, 0.35),
            missing_token_fallback: rgba(255, 0, 255),
            tokens: phf::Map::new(),
            runtime_tokens: crate::grid::tokens(),
        }
    }

    /// Resolve the preferred control width for a semantic size preset.
    #[must_use]
    pub fn control_width(&self, size: Size) -> Dimension {
        self.dimension_token(
            match size {
                Size::Compact => "control-width-compact",
                Size::Default => "control-width-default",
                Size::Wide => "control-width-wide",
                Size::Grow => return Dimension::Full,
            },
            match size {
                Size::Compact => 24,
                Size::Default => 32,
                Size::Wide => 148,
                Size::Grow => unreachable!(),
            },
        )
    }

    /// Resolve the preferred control height for a semantic size preset.
    #[must_use]
    pub fn control_height(&self, size: Size) -> Dimension {
        self.dimension_token(
            match size {
                Size::Compact => "control-height-compact",
                Size::Default => "control-height-default",
                Size::Wide => "control-height-wide",
                Size::Grow => return Dimension::Auto,
            },
            match size {
                Size::Compact => 22,
                Size::Default => 28,
                Size::Wide => 36,
                Size::Grow => unreachable!(),
            },
        )
    }

    /// Resolve uniform padding for a semantic size preset.
    #[must_use]
    pub fn padding(&self, size: Size) -> LengthPercentage {
        self.length_token(
            match size {
                Size::Compact => "padding-compact",
                Size::Default => "padding-default",
                Size::Wide => "padding-wide",
                Size::Grow => return LengthPercentage::Percent(1.0),
            },
            match size {
                Size::Compact => 6,
                Size::Default => 10,
                Size::Wide => 14,
                Size::Grow => unreachable!(),
            },
        )
    }

    /// Resolve uniform gap for a semantic size preset.
    #[must_use]
    pub fn gap(&self, size: Size) -> LengthPercentage {
        self.length_token(
            match size {
                Size::Compact => "gap-compact",
                Size::Default => "gap-default",
                Size::Wide => "gap-wide",
                Size::Grow => return LengthPercentage::Percent(1.0),
            },
            match size {
                Size::Compact => 4,
                Size::Default => 8,
                Size::Wide => 12,
                Size::Grow => unreachable!(),
            },
        )
    }

    /// Resolve border radius for a semantic size preset.
    #[must_use]
    pub fn radius(&self, size: Size) -> LengthPercentage {
        self.length_token(
            match size {
                Size::Compact => "radius-compact",
                Size::Default => "radius-default",
                Size::Wide => "radius-wide",
                Size::Grow => return LengthPercentage::Percent(1.0),
            },
            match size {
                Size::Compact => 2,
                Size::Default => 4,
                Size::Wide => 8,
                Size::Grow => unreachable!(),
            },
        )
    }

    fn dimension_token(&self, token: &'static str, fallback: i16) -> Dimension {
        Dimension::Px(self.px_token(token, fallback))
    }

    fn length_token(&self, token: &'static str, fallback: i16) -> LengthPercentage {
        LengthPercentage::Px(self.px_token(token, fallback))
    }

    fn px_token(&self, token: &'static str, fallback: i16) -> Px {
        if let Some(ThemeValue::Px(px)) = self.runtime_tokens.get(token).copied() {
            return px_from_f32(px);
        }
        match self.tokens.get(token).copied() {
            Some(ThemeValue::Px(px)) => px_from_f32(px),
            _ => Px(fallback),
        }
    }
    /// The non-terminal regression baseline — Obsidian palette under
    /// the Normal shape variant. The default Sigil look.
    #[must_use]
    pub fn normal() -> Self {
        Self::from_palette(Palette::obsidian(), Variant::Normal)
    }

    /// The terminal regression baseline — Terminal palette under the
    /// Terminal shape variant. Green-on-black monospace stress-test.
    #[must_use]
    pub fn terminal() -> Self {
        Self::from_palette(Palette::terminal(), Variant::Terminal)
    }
}

const fn is_dark(c: Rgba8) -> bool {
    // Cheap perceived-brightness gate — bright pixels return false.
    // 30R + 59G + 11B / 100 < 128 ⇒ dark.
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    (r * 30 + g * 59 + b * 11) < 12_800
}

/// Returns the candidate (`a` or `b`) whose perceived-brightness sits
/// further from `against` on the gamma-encoded BT.601 axis.
///
/// Used by [`Theme::from_palette`] to pick `on_accent` text colour:
/// against a dark accent the brighter of the two palette colours wins,
/// and vice versa. Gamma-uncorrected — adequate for the binary "which
/// is more contrasty" decision but **not** a WCAG contrast ratio.
const fn pick_higher_contrast(against: Rgba8, a: Rgba8, b: Rgba8) -> Rgba8 {
    let target = perceived_brightness_u32(against);
    let da = perceived_brightness_u32(a).abs_diff(target);
    let db = perceived_brightness_u32(b).abs_diff(target);
    if da >= db { a } else { b }
}

const fn perceived_brightness_u32(c: Rgba8) -> u32 {
    // 30R + 59G + 11B (unscaled). Range 0..=25500 for any u8 input.
    let r = c.r as u32;
    let g = c.g as u32;
    let b = c.b as u32;
    r * 30 + g * 59 + b * 11
}

/// Shift a colour's per-channel brightness by `delta`, clamping to the
/// 0..=255 range. Used for hover lifts.
fn shift(c: Rgba8, delta: i16) -> Rgba8 {
    let nudge = |channel: u8| -> u8 {
        // `clamp(0, 255)` constrains the value to `0..=255`, so the
        // downcast is exact — documents the invariant rather than
        // masking a real lossy cast.
        #[allow(
            clippy::cast_sign_loss,
            reason = "Clamped to 0..=255 before cast; no sign to lose."
        )]
        {
            i16::from(channel).saturating_add(delta).clamp(0, 255) as u8
        }
    };
    Rgba8 {
        r: nudge(c.r),
        g: nudge(c.g),
        b: nudge(c.b),
        a: c.a,
    }
}

/// 12-step accent scale from a theme-file solid. Steps 1–8 climb from
/// `surface` to the solid; 9 is the solid; 10–12 lift toward white.
fn ramp_accent(surface: Rgba8, solid: Rgba8) -> crate::scale::ColorScale {
    let mut steps = [solid; 12];
    for (step, t) in steps.iter_mut().zip(1..=9u8).take(8) {
        *step = alpha_mix(surface, solid, f32::from(t) / 9.0);
    }
    steps[8] = solid;
    steps[9] = alpha_mix(
        solid,
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        0.18,
    );
    steps[10] = alpha_mix(
        solid,
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        0.36,
    );
    steps[11] = alpha_mix(
        solid,
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        0.55,
    );
    crate::scale::ColorScale::from_steps(steps).with_on_solid(surface)
}

/// Linear-RGB lerp between `a` and `b` using `t ∈ [0, 1]` as the
/// weight on `b`. Used to derive subtle border colours from a
/// foreground+background pair.
fn alpha_mix(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| -> u8 {
        let xf = f32::from(x);
        let yf = f32::from(y);
        let mixed = (1.0 - t).mul_add(xf, t * yf).round().clamp(0.0, 255.0);
        // `mixed` is rounded then clamped to `0.0..=255.0` — the cast
        // to `u8` is exact for every representable value. Document
        // the proof rather than mask a real lossy cast.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Value clamped to 0.0..=255.0 before cast — bounded."
        )]
        {
            mixed as u8
        }
    };
    Rgba8 {
        r: blend(a.r, b.r),
        g: blend(a.g, b.g),
        b: blend(a.b, b.b),
        a: a.a,
    }
}

/// Convenience access to the [`FontFamilyHint`] a [`Theme`] inherited
/// from its [`Variant`]. Renderers consult this when picking a face.
#[must_use]
pub const fn font_family_hint(variant: Variant) -> FontFamilyHint {
    variant.tokens().font
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
    #[allow(
        clippy::missing_const_for_fn,
        reason = "Theme now owns a HashMap; const-stable in the consumer regardless."
    )]
    pub fn tokens(mut self, tokens: phf::Map<&'static str, ThemeValue>) -> Self {
        self.theme.tokens = tokens;
        self
    }

    /// Register a runtime token. Overlays compile-time `phf` tokens.
    #[must_use]
    pub fn token(mut self, name: impl Into<String>, value: ThemeValue) -> Self {
        self.theme.runtime_tokens.insert(name.into(), value);
        self
    }

    /// Build the theme.
    #[must_use]
    pub fn build(self) -> Theme {
        self.theme
    }
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

fn px_from_f32(value: f32) -> Px {
    let px = value
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX));
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Rounded and clamped to the i16 range before cast."
    )]
    {
        Px(px as i16)
    }
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

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
