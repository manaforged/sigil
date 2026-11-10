use crate::text::font_family::FontFaces;
use crate::text::fonts::DefaultFontSource;
use crate::text::shaper::{Font, Script};
use smallvec::SmallVec;

const OBLIQUE_SKEW: f32 = 0.2;

pub struct FontCollection {
    sans: FontFaces,
    mono: FontFaces,
    serif: Option<FontFaces>,
    display: Option<FontFaces>,
    icon: Option<Font>,
    pub(crate) fallbacks: Vec<Font>,
    emoji: Font,
    symbols: Option<Font>,
}

impl core::fmt::Debug for FontCollection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FontCollection")
            .field("icon", &self.icon)
            .field("fallbacks", &self.fallbacks)
            .field("emoji", &self.emoji)
            .field("symbols", &self.symbols)
            .finish_non_exhaustive()
    }
}

impl FontCollection {
    #[must_use]
    pub fn with_default() -> Self {
        let source = crate::text::fonts::default_font_source();
        #[cfg(feature = "desktop")]
        if matches!(source, DefaultFontSource::System) {
            return Self::from_system();
        }
        let over = match source {
            #[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
            DefaultFontSource::Custom(fonts) => Some(fonts),
            DefaultFontSource::Bundled => None,
            #[cfg(feature = "desktop")]
            DefaultFontSource::System => None,
        };
        let face = |slot: Option<&(std::sync::Arc<Vec<u8>>, u32)>| -> Option<Font> {
            let (bytes, index) = slot?;
            Font::from_shared(std::sync::Arc::clone(bytes), *index).ok()
        };
        Self {
            sans: sans_family(over.as_ref()),
            mono: FontFaces::new(
                over.as_ref()
                    .and_then(|f| face(f.mono.as_ref()))
                    .unwrap_or_else(load_mono_font),
            ),
            serif: None,
            display: over
                .as_ref()
                .and_then(|f| face(f.display.as_ref()))
                .map(FontFaces::new),
            icon: over.as_ref().and_then(|f| face(f.icon.as_ref())),
            fallbacks: crate::text::fonts::added_fallback_fonts(),
            emoji: load_emoji_font(),
            symbols: load_symbols_font(),
        }
    }

    #[cfg(feature = "desktop")]
    fn from_system() -> Self {
        let system = crate::text::system_fonts::load();
        Self {
            sans: system
                .sans
                .unwrap_or_else(|| FontFaces::new(load_sans_font())),
            mono: system
                .mono
                .unwrap_or_else(|| FontFaces::new(load_mono_font())),
            serif: system.serif,
            display: None,
            icon: None,
            fallbacks: crate::text::fonts::added_fallback_fonts(),
            emoji: load_emoji_font(),
            symbols: load_symbols_font(),
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
    pub(crate) fn select_weight(&self, family: &str, weight: u16) -> &Font {
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

    #[cfg(any(test, feature = "kittest"))]
    pub fn register(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), crate::Error> {
        let font = Font::from_bytes(bytes, face_index)?;
        self.sans.replace(font);
        Ok(())
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn register_family(
        &mut self,
        family: &str,
        bytes: Vec<u8>,
        face_index: u32,
    ) -> Result<(), crate::Error> {
        let font = Font::from_bytes(bytes, face_index)?;
        let name = family.trim();
        if name.eq_ignore_ascii_case("sans")
            || name.eq_ignore_ascii_case("sans-serif")
            || name.eq_ignore_ascii_case("system-ui")
        {
            self.sans.replace(font);
        } else if name.eq_ignore_ascii_case("mono") || name.eq_ignore_ascii_case("monospace") {
            self.mono.replace(font);
        } else if name.eq_ignore_ascii_case("serif") {
            self.serif = Some(FontFaces::new(font));
        } else if name.eq_ignore_ascii_case("icon") {
            self.icon = Some(font);
        } else if name.eq_ignore_ascii_case("display") {
            self.display = Some(FontFaces::new(font));
        } else {
            return Err(crate::Error::fixed(
                crate::ErrorKind::Font,
                "invalid font data",
            ));
        }
        Ok(())
    }

    #[cfg(any(test, feature = "kittest"))]
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
            .chain(self.symbols.as_ref())
            .chain(std::iter::once(&self.emoji))
            .chain(&system);
        for candidate in candidates {
            if chain.iter().all(|f| f.id() != candidate.id()) {
                chain.push(candidate.clone());
            }
        }
        chain
    }
}

pub(crate) type ChainScripts = SmallVec<[Script; 4]>;

fn chain_script(ch: char) -> Script {
    let symbol = matches!(
        ch,
        '\u{2300}'..='\u{23ff}'
            | '\u{2600}'..='\u{27bf}'
            | '\u{2b00}'..='\u{2bff}'
            | '\u{1fb00}'..='\u{1fbff}'
    );
    if symbol {
        Script::Other
    } else {
        super::char_script(ch)
    }
}

pub(crate) fn chain_scripts(text: &str) -> ChainScripts {
    let mut scripts = ChainScripts::new();
    for script in text.chars().map(chain_script) {
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

fn load_symbols_font() -> Option<Font> {
    Font::from_bytes(sigil_fonts::SIGIL_SYMBOLS.to_vec(), 0).ok()
}

fn load_mono_font() -> Font {
    const BYTES: &[u8] = sigil_fonts::CASCADIA_MONO_REGULAR;
    Font::from_bytes(BYTES.to_vec(), 0).expect("bundled CascadiaMono-Regular.ttf must parse")
}

fn sans_family(over: Option<&crate::text::fonts::DefaultFonts>) -> FontFaces {
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
    FontFaces::from_weighted(faces).unwrap_or_else(|| FontFaces::new(load_sans_font()))
}
