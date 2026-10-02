#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]
use crate::text::error::TextError;
use crate::text::font_family::FontFamily;
use crate::text::fonts::DefaultFontSource;
use crate::text::shaper::{Font, Script};
use smallvec::SmallVec;

const OBLIQUE_SKEW: f32 = 0.2;

pub struct FontCollection {
    sans: FontFamily,
    mono: FontFamily,
    serif: Option<FontFamily>,
    display: Option<FontFamily>,
    icon: Option<Font>,
    pub(crate) fallbacks: Vec<Font>,
    emoji: Font,
}

impl FontCollection {
    #[must_use]
    pub fn with_default() -> Self {
        let source = crate::text::fonts::default_font_source();
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
            sans: sans_family(over.as_ref()),
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
            icon: over.as_ref().and_then(|f| face(f.icon.as_ref())),
            fallbacks: over
                .as_ref()
                .map(|f| {
                    f.fallbacks
                        .iter()
                        .filter_map(|slot| face(Some(slot)))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
                .into_iter()
                .chain(crate::text::fonts::added_fallback_fonts())
                .collect(),
            emoji: load_emoji_font(),
        }
    }

    fn from_system() -> Self {
        let system = crate::text::system_fonts::load();
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
            fallbacks: crate::text::fonts::added_fallback_fonts(),
            emoji: load_emoji_font(),
        }
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn primary(&self) -> &Font {
        self.sans.select(400)
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn select(&self, family: &str) -> &Font {
        self.select_weight(family, 400)
    }

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

    #[must_use]
    pub(crate) fn select_face(&self, family: &str, weight: u16, italic: bool) -> (&Font, f32) {
        let font = self.select_weight(family, weight);
        (font, if italic { OBLIQUE_SKEW } else { 0.0 })
    }

    pub fn register(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), TextError> {
        let font = Font::from_bytes(bytes, face_index).map_err(|_| TextError::InvalidFont)?;
        self.sans.replace(font);
        Ok(())
    }

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

    #[must_use]
    pub(crate) fn fallback_chain(&self, primary: &Font, weight: u16, text: &str) -> Vec<Font> {
        self.chain_for(primary, weight, &chain_scripts(text))
    }

    #[must_use]
    pub(crate) fn chain_for(&self, primary: &Font, weight: u16, scripts: &[Script]) -> Vec<Font> {
        let mut chain: Vec<Font> = vec![primary.clone()];
        let system: Vec<Font> = scripts
            .iter()
            .copied()
            .flat_map(crate::text::system_fonts::script_fallbacks)
            .collect();
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
            .chain(&system)
            .chain(std::iter::once(&self.emoji));
        for candidate in candidates {
            if chain.iter().all(|f| f.id() != candidate.id()) {
                chain.push(candidate.clone());
            }
        }
        chain
    }
}

pub(crate) type ChainScripts = SmallVec<[Script; 4]>;

pub(crate) fn chain_scripts(text: &str) -> ChainScripts {
    let mut scripts = ChainScripts::new();
    for script in text.chars().map(super::char_script) {
        if !crate::text::script_fonts::candidates(script).is_empty() && !scripts.contains(&script) {
            scripts.push(script);
        }
    }
    scripts
}

fn load_sans_font() -> Font {
    const BYTES: &[u8] = sigil_fonts::FIRA_SANS_REGULAR;
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled FiraSans-Regular.ttf must parse — fixture validated at G1.3")
}

fn load_sans_medium_font() -> Font {
    const BYTES: &[u8] = sigil_fonts::FIRA_SANS_MEDIUM;
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled FiraSans-Medium.ttf must parse — fixture validated with Regular")
}

fn load_sans_semibold_font() -> Font {
    const BYTES: &[u8] = sigil_fonts::FIRA_SANS_SEMIBOLD;
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled FiraSans-SemiBold.ttf must parse — fixture validated with Regular")
}

fn load_emoji_font() -> Font {
    const BYTES: &[u8] = sigil_fonts::NOTO_COLOR_EMOJI_SUBSET;
    Font::from_bytes(BYTES.to_vec(), 0)
        .expect("bundled NotoColorEmoji-subset.ttf must parse — validated in color_glyph tests")
}

fn load_mono_font() -> Font {
    const BYTES: &[u8] = sigil_fonts::CASCADIA_MONO_REGULAR;
    Font::from_bytes(BYTES.to_vec(), 0).expect("bundled CascadiaMono-Regular.ttf must parse")
}

fn sans_family(over: Option<&crate::text::fonts::DefaultFonts>) -> FontFamily {
    let mut faces: Vec<(u16, Font)> = Vec::new();
    if let Some(font) = over
        .and_then(|f| f.sans.as_ref())
        .and_then(|(bytes, index)| Font::from_shared(std::sync::Arc::clone(bytes), *index).ok())
    {
        faces.push((400, font));
    } else {
        faces.push((400, load_sans_font()));
        faces.push((500, load_sans_medium_font()));
        faces.push((600, load_sans_semibold_font()));
    }
    if let Some(fonts) = over {
        for (weight, bytes, index) in &fonts.sans_weights {
            if let Ok(font) = Font::from_shared(std::sync::Arc::clone(bytes), *index) {
                faces.push((*weight, font));
            }
        }
    }
    FontFamily::from_weighted(faces).unwrap_or_else(|| FontFamily::new(load_sans_font()))
}
