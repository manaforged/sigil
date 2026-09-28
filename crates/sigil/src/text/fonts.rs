use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

pub(crate) type SansWeight = (u16, Arc<Vec<u8>>, u32);

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

type FontsChanged = Arc<dyn Fn() + Send + Sync>;
static ON_CHANGE: RwLock<Option<FontsChanged>> = RwLock::new(None);

#[cfg(feature = "desktop")]
pub(crate) fn on_fonts_changed(notify: impl Fn() + Send + Sync + 'static) {
    if let Ok(mut guard) = ON_CHANGE.write() {
        *guard = Some(Arc::new(notify));
    }
}

fn fonts_changed() {
    GENERATION.fetch_add(1, Ordering::Release);
    wake_font_listeners();
}

pub(crate) fn wake_font_listeners() {
    let notify = ON_CHANGE.read().ok().and_then(|guard| guard.clone());
    if let Some(notify) = notify {
        notify();
    }
}

pub fn set_default_fonts(fonts: DefaultFonts) {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::Custom(fonts);
        drop(guard);
        fonts_changed();
    }
}

pub fn use_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::System;
        drop(guard);
        fonts_changed();
    }
}

pub fn prefer_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write()
        && matches!(*guard, DefaultFontSource::Bundled)
    {
        *guard = DefaultFontSource::System;
        drop(guard);
        fonts_changed();
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
