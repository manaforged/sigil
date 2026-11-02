use super::AppConfig;
#[cfg(feature = "scene")]
use std::sync::Arc;

impl AppConfig {
    /// Replaces the bundled sans face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_sans(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.fonts.get_or_insert_default().sans = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Adds a sans face for one font weight.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_sans_weight(mut self, weight: u16, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        let entry = (weight, Arc::new(bytes.into()), face);
        self.fonts.get_or_insert_default().sans_weights.push(entry);
        self
    }

    /// Replaces the bundled monospace face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_mono(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.fonts.get_or_insert_default().mono = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Replaces the bundled icon face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_icon(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.fonts.get_or_insert_default().icon = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Replaces the bundled display face.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_display(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        self.fonts.get_or_insert_default().display = Some((Arc::new(bytes.into()), face));
        self
    }

    /// Adds a fallback face and keeps the primary font source.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_fallback(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        let entry = (Arc::new(bytes.into()), face);
        self.font_fallbacks.push(entry);
        self
    }

    /// Turns glyph dilation and sharpening on or off.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn text_enhance(mut self, yes: bool) -> Self {
        self.text_tuning.get_or_insert_default().enabled = yes;
        self
    }

    /// Turns pixel snapping of glyphs on or off.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn text_snap(mut self, yes: bool) -> Self {
        self.text_tuning.get_or_insert_default().snap = yes;
        self
    }

    /// Selects atlas rasterization for small text instead of the vector path.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn text_raster(mut self, yes: bool) -> Self {
        self.text_tuning.get_or_insert_default().raster = yes;
        self
    }

    /// Sets the glyph dilation.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn text_dilation(mut self, dilation: f32) -> Self {
        self.text_tuning.get_or_insert_default().dilation = dilation;
        self
    }

    /// Sets the glyph sharpening.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn text_sharpen(mut self, sharpen: f32) -> Self {
        self.text_tuning.get_or_insert_default().sharpen = sharpen;
        self
    }

    /// Sets the text gamma for light text on dark and dark text on light.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn text_gamma(mut self, light_on_dark: f32, dark_on_light: f32) -> Self {
        let tuning = self.text_tuning.get_or_insert_default();
        tuning.gamma_light_on_dark = light_on_dark;
        tuning.gamma_dark_on_light = dark_on_light;
        self
    }
}
