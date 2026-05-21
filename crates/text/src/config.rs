//! Default text configuration.

use style::{TextLineHeight, TextStyle};

/// Default font and style settings used when no per-node style is set.
#[derive(Clone, Debug)]
pub struct TextConfig {
    /// Default font family name (e.g. `"system-ui"`).
    pub font_family: String,
    /// Default font size in logical pixels.
    pub font_size: f32,
    /// Default CSS-style font weight.
    pub font_weight: u16,
    /// Line height as a multiplier of `font_size`.
    pub line_height: f32,
}

impl TextConfig {
    /// Convert config defaults into resolved text style.
    #[must_use]
    pub fn text_style(&self) -> TextStyle {
        TextStyle {
            font_family: self.font_family.clone(),
            font_size: self.font_size.max(1.0),
            font_weight: self.font_weight,
            line_height: TextLineHeight::FontSizeRelative(self.line_height.max(0.1)),
        }
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 16.0,
            font_weight: 400,
            line_height: 1.2,
        }
    }
}
