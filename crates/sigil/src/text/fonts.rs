use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

pub type SansWeight = (u16, std::sync::Arc<Vec<u8>>, u32);

#[derive(Clone, Default)]
pub struct DefaultFonts {
    pub sans: Option<(Arc<Vec<u8>>, u32)>,
    pub sans_weights: Vec<SansWeight>,
    pub mono: Option<(Arc<Vec<u8>>, u32)>,
    pub icon: Option<(Arc<Vec<u8>>, u32)>,
    pub display: Option<(Arc<Vec<u8>>, u32)>,
    pub fallbacks: Vec<(Arc<Vec<u8>>, u32)>,
}

#[derive(Clone, Default)]
pub(crate) enum DefaultFontSource {
    #[default]
    Bundled,
    Custom(DefaultFonts),
    System,
}

static DEFAULT_FONTS: RwLock<DefaultFontSource> = RwLock::new(DefaultFontSource::Bundled);
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn set_default_fonts(fonts: DefaultFonts) {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::Custom(fonts);
        GENERATION.fetch_add(1, Ordering::Release);
    }
}

pub fn use_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::System;
        GENERATION.fetch_add(1, Ordering::Release);
    }
}

#[doc(hidden)]
pub fn prefer_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write()
        && matches!(*guard, DefaultFontSource::Bundled)
    {
        *guard = DefaultFontSource::System;
        GENERATION.fetch_add(1, Ordering::Release);
    }
}

pub(crate) fn default_font_source() -> DefaultFontSource {
    DEFAULT_FONTS
        .read()
        .map_or_else(|_| DefaultFontSource::Bundled, |guard| guard.clone())
}

pub(crate) fn default_fonts_generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}
