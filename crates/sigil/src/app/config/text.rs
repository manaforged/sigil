use super::AppConfig;
#[cfg(feature = "scene")]
use crate::style::FontFamily;
#[cfg(feature = "scene")]
use std::sync::Arc;

/// The fonts that an app config carries.
#[cfg(feature = "scene")]
#[derive(Debug, Clone, Default)]
pub(crate) struct TextSetup {
    pub(crate) fonts: Option<crate::text::fonts::DefaultFonts>,
    pub(crate) font_fallbacks: Vec<crate::text::fonts::FallbackFace>,
    pub(crate) web_fonts: Option<Vec<crate::text::WebFont>>,
}

impl AppConfig {
    /// Replaces the bundled face of `family` with face `face` of the font file `bytes`; `Serif` has no bundled face and is ignored.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font(mut self, family: FontFamily, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        let fonts = self.text.fonts.get_or_insert_default();
        let slot = match family {
            FontFamily::Sans => &mut fonts.sans,
            FontFamily::Mono => &mut fonts.mono,
            FontFamily::Icon => &mut fonts.icon,
            FontFamily::Display => &mut fonts.display,
            FontFamily::Serif => {
                tracing::warn!(target: "sigil::text", "the serif family has no bundled face to replace");
                return self;
            }
        };
        *slot = Some((Arc::new(bytes.into()), face));
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

    /// Adds a fallback face and keeps the primary font source.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn font_fallback(mut self, bytes: impl Into<Vec<u8>>, face: u32) -> Self {
        let entry = (Arc::new(bytes.into()), face);
        self.text.font_fallbacks.push(entry);
        self
    }

    /// Replaces the fonts that a browser build fetches for scripts the bundled faces lack.
    #[cfg(feature = "scene")]
    #[must_use]
    pub fn web_fonts(mut self, fonts: impl IntoIterator<Item = crate::text::WebFont>) -> Self {
        self.text.web_fonts = Some(fonts.into_iter().collect());
        self
    }
}
