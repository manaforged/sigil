//! Owned font collection: bootstrap faces, overrides, and the
//! per-run fallback chain.

use crate::error::TextError;
use crate::font_family::FontFamily;
use crate::fonts::DefaultFontSource;
use crate::shaper::Font;

/// Owned font collection.
///
/// Bootstrapped with the bundled `FiraSans` Regular face as the sans
/// primary and `CascadiaMono` Regular as the monospace face. Callers
/// can register additional families via [`Self::register_family`].
pub struct FontCollection {
    sans: FontFamily,
    mono: FontFamily,
    serif: Option<FontFamily>,
    display: Option<FontFamily>,
    icon: Option<Font>,
    /// Coverage-only fallback faces, in registration order. Consulted by
    /// the per-run chain after the named families, before emoji.
    pub(crate) fallbacks: Vec<Font>,
    /// Bundled Noto Color Emoji subset — the last-resort fallback face so
    /// emoji codepoints no text face covers resolve to a color glyph
    /// instead of tofu. Rendered through [`crate::color_glyph`], not Slug.
    emoji: Font,
}

impl FontCollection {
    /// Construct with the bootstrap sans + mono faces.
    ///
    /// Honors any process-global override registered via
    /// [`set_default_fonts`](crate::set_default_fonts) before falling back to
    /// the embedded `FiraSans` + `CascadiaMono` faces — including when an
    /// override's bytes fail to parse, so a malformed consumer font can never
    /// blank the UI.
    #[must_use]
    pub fn with_default() -> Self {
        let source = crate::fonts::default_font_source();
        if matches!(source, DefaultFontSource::System) {
            return Self::from_system();
        }
        let over = match source {
            DefaultFontSource::Custom(fonts) => Some(fonts),
            DefaultFontSource::Bundled | DefaultFontSource::System => None,
        };
        let face = |slot: Option<&(std::sync::Arc<Vec<u8>>, u32)>| -> Option<Font> {
            let (bytes, index) = slot?;
            Font::from_shared(std::sync::Arc::clone(bytes), *index).ok()
        };
        Self {
            sans: FontFamily::new(
                over.as_ref()
                    .and_then(|f| face(f.sans.as_ref()))
                    .unwrap_or_else(load_sans_font),
            ),
            mono: FontFamily::new(
                over.as_ref()
                    .and_then(|f| face(f.mono.as_ref()))
                    .unwrap_or_else(load_mono_font),
            ),
            serif: None,
            display: over
                .as_ref()
                .and_then(|f| face(f.display.as_ref()))
                .map(FontFamily::new),
            // No bundled icon face — `select("icon")` falls back to sans until a
            // consumer registers one (e.g. the host app's codicon) via the
            // `DefaultFonts.icon` override.
            icon: over.as_ref().and_then(|f| face(f.icon.as_ref())),
            fallbacks: over
                .as_ref()
                .map(|f| {
                    f.fallbacks
                        .iter()
                        .filter_map(|slot| face(Some(slot)))
                        .collect()
                })
                .unwrap_or_default(),
            emoji: load_emoji_font(),
        }
    }

    /// The native desktop UI families, bundled faces filling any gap.
    fn from_system() -> Self {
        let system = crate::system_fonts::load();
        Self {
            sans: system
                .sans
                .unwrap_or_else(|| FontFamily::new(load_sans_font())),
            mono: system
                .mono
                .unwrap_or_else(|| FontFamily::new(load_mono_font())),
            serif: system.serif,
            display: None,
            icon: None,
            fallbacks: Vec::new(),
            emoji: load_emoji_font(),
        }
    }

    /// Borrow the sans/primary face. Retained for compatibility with
    /// callers that don't yet pick a family — equivalent to `select("sans")`.
    #[must_use]
    pub fn primary(&self) -> &Font {
        self.sans.select(400)
    }

    /// Resolve a family name to a font handle. Unknown families fall
    /// back to the sans primary so unstyled callers always render
    /// something legible. Case-insensitive on the well-known names.
    #[must_use]
    pub fn select(&self, family: &str) -> &Font {
        self.select_weight(family, 400)
    }

    /// Resolve a family and CSS weight to the matching static or variable face.
    #[must_use]
    pub fn select_weight(&self, family: &str, weight: u16) -> &Font {
        let name = family.trim();
        if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            self.mono.select(weight)
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif.as_ref().unwrap_or(&self.sans).select(weight)
        } else if name.eq_ignore_ascii_case("icon") {
            self.icon
                .as_ref()
                .unwrap_or_else(|| self.sans.select(weight))
        } else if name.eq_ignore_ascii_case("display") {
            self.display.as_ref().unwrap_or(&self.sans).select(weight)
        } else {
            self.sans.select(weight)
        }
    }

    /// Replace the sans/primary face with a user-supplied font.
    ///
    /// # Errors
    ///
    /// Returns [`TextError::InvalidFont`] when the bytes do not parse as
    /// a valid OpenType/TrueType face.
    pub fn register(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), TextError> {
        let font = Font::from_bytes(bytes, face_index).map_err(|_| TextError::InvalidFont)?;
        self.sans.replace(font);
        Ok(())
    }

    /// Replace the face for a named family. Recognised names:
    /// `sans` / `sans-serif` / `system-ui`, `mono` / `monospace`,
    /// `serif`. Unknown names return [`TextError::InvalidFont`].
    ///
    /// # Errors
    ///
    /// Returns [`TextError::InvalidFont`] when the bytes are unparseable
    /// or the family name is unknown.
    pub fn register_family(
        &mut self,
        family: &str,
        bytes: Vec<u8>,
        face_index: u32,
    ) -> Result<(), TextError> {
        let font = Font::from_bytes(bytes, face_index).map_err(|_| TextError::InvalidFont)?;
        let name = family.trim();
        if name.eq_ignore_ascii_case("sans")
            || name.eq_ignore_ascii_case("sans-serif")
            || name.eq_ignore_ascii_case("system-ui")
        {
            self.sans.replace(font);
        } else if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            self.mono.replace(font);
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif = Some(FontFamily::new(font));
        } else if name.eq_ignore_ascii_case("icon") {
            self.icon = Some(font);
        } else if name.eq_ignore_ascii_case("display") {
            self.display = Some(FontFamily::new(font));
        } else {
            return Err(TextError::InvalidFont);
        }
        Ok(())
    }

    /// Deterministic per-run fallback order for `primary`: `primary`
    /// itself first, then the collection's other distinct registered
    /// faces (sans, mono, serif, icon, in that fixed order) so a
    /// cluster missing from `primary` can still find coverage instead
    /// of rendering `.notdef` tofu (Wave 4 fallback chain). Faces are
    /// deduped by identity — registering the same bytes under two
    /// family names doesn't repeat them in the chain. `Font::clone` is
    /// an `Arc` bump, so the chain is cheap to build once per paragraph.
    #[must_use]
    pub(crate) fn fallback_chain(&self, primary: &Font, weight: u16) -> Vec<Font> {
        let mut chain: Vec<Font> = vec![primary.clone()];
        let candidates = [
            Some(self.sans.select(weight)),
            Some(self.mono.select(weight)),
            self.serif.as_ref().map(|family| family.select(weight)),
            self.icon.as_ref(),
        ];
        let candidates = candidates
            .into_iter()
            .flatten()
            .chain(&self.fallbacks)
            // Emoji last: only reached for codepoints no text face covers.
            .chain(std::iter::once(&self.emoji));
        for candidate in candidates {
            if chain.iter().all(|f| f.id() != candidate.id()) {
                chain.push(candidate.clone());
            }
        }
        chain
    }
}

/// Embedded `FiraSans` Regular — sans bootstrap face.
fn load_sans_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../../assets/fonts/FiraSans-Regular.ttf");
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled FiraSans-Regular.ttf must parse — fixture validated at G1.3")
}

fn load_emoji_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../../assets/fonts/NotoColorEmoji-subset.ttf");
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled NotoColorEmoji-subset.ttf must parse — validated in color_glyph tests")
}

/// Embedded `CascadiaMono` Regular — monospace bootstrap face. OFL
/// 1.1 (Microsoft, <https://github.com/microsoft/cascadia-code>).
fn load_mono_font() -> Font {
    const BYTES: &[u8] = include_bytes!("../../assets/fonts/CascadiaMono-Regular.ttf");
    Font::from_bytes(BYTES.to_vec(), 0).expect("bundled CascadiaMono-Regular.ttf must parse")
}
