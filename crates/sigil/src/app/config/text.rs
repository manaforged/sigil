use super::AppConfig;
#[cfg(feature = "scene")]
use std::sync::Arc;

/// The fonts that an app config carries.
#[cfg(feature = "scene")]
#[derive(Debug, Clone, Default)]
pub(crate) struct TextSetup {
    pub(crate) fonts: Option<crate::text::fonts::DefaultFonts>,
    pub(crate) font_fallbacks: Vec<crate::text::fonts::FallbackFace>,
}

impl AppConfig {
    /// Replaces the bundled sans face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_sans(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.text.fonts.get_or_insert_default().sans = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Adds a sans face for one font weight.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_sans_weight(mut self, weight: u16, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        let entry = (weight, Arc::new(bytes.into()), face);
        self.text
            .fonts
            .get_or_insert_default()
            .sans_weights
            .push(entry);
        self
    }

    /// Replaces the bundled monospace face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_mono(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.text.fonts.get_or_insert_default().mono = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Replaces the bundled icon face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_icon(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.text.fonts.get_or_insert_default().icon = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Replaces the bundled display face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_display(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.text.fonts.get_or_insert_default().display = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Adds a fallback face and keeps the primary font source.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_fallback(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        let entry = (Arc::new(bytes.into()), face);
        self.text.font_fallbacks.push(entry);
        self
    }
}
