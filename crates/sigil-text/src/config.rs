//! Default text configuration.

/// Default font and style settings used when no per-node style is set.
///
/// This becomes the future attachment point for per-node styles
/// (DR-006 / `sigil-style`) without breaking changes.
#[derive(Clone, Debug)]
pub struct TextConfig {
    /// Default font family name (e.g. `"system-ui"`).
    pub font_family: String,
    /// Default font size in logical pixels.
    pub font_size: f32,
    /// Line height as a multiplier of `font_size`.
    pub line_height: f32,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            font_family: String::from("system-ui"),
            font_size: 16.0,
            line_height: 1.2,
        }
    }
}
