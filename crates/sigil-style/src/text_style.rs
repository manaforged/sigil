//! Resolved typography used by text measurement and painting.

use sigil_hal::TextEngine;

/// Resolved line-height value for paragraph layout.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextLineHeight {
    /// Unitless multiple of the font size.
    FontSizeRelative(f32),
    /// Absolute logical-pixel line height.
    Absolute(f32),
}

impl Default for TextLineHeight {
    fn default() -> Self {
        Self::FontSizeRelative(1.2)
    }
}

/// Resolved typography used by text measurement and painting.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// Font family name, e.g. `"system-ui"`.
    pub font_family: String,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// CSS-style font weight.
    pub font_weight: u16,
    /// Paragraph line-height.
    pub line_height: TextLineHeight,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 16.0,
            font_weight: 400,
            line_height: TextLineHeight::default(),
        }
    }
}

/// Text engines that can apply resolved Sigil typography.
pub trait StyledTextEngine: TextEngine {
    /// Lay out a paragraph with explicit resolved typography.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if paragraph layout fails.
    fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Self::Paragraph, Self::Error>;
}
