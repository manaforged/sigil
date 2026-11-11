use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::text::shaper::Font;

pub(crate) type SansWeight = (u16, Arc<Vec<u8>>, u32);

#[derive(Clone, Default)]
pub struct DefaultFonts {
    pub sans: Option<(Arc<Vec<u8>>, u32)>,
    pub sans_weights: Vec<SansWeight>,
    pub mono: Option<(Arc<Vec<u8>>, u32)>,
    pub icon: Option<(Arc<Vec<u8>>, u32)>,
    pub display: Option<(Arc<Vec<u8>>, u32)>,
}

impl core::fmt::Debug for DefaultFonts {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DefaultFonts").finish_non_exhaustive()
    }
}

#[derive(Clone, Default)]
pub(crate) enum DefaultFontSource {
    #[default]
    Bundled,
    #[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
    Custom(DefaultFonts),
    #[cfg(feature = "desktop")]
    System,
}

static DEFAULT_FONTS: RwLock<DefaultFontSource> = RwLock::new(DefaultFontSource::Bundled);
static GENERATION: AtomicU64 = AtomicU64::new(0);
pub(crate) type FallbackFace = (Arc<Vec<u8>>, u32);

static ADDED_FALLBACKS: RwLock<Vec<FallbackFace>> = RwLock::new(Vec::new());

#[cfg(any(
    test,
    target_arch = "wasm32",
    feature = "desktop",
    feature = "kittest",
    feature = "terminal"
))]
type FontsChanged = Arc<dyn Fn() + Send + Sync>;
#[cfg(any(
    test,
    target_arch = "wasm32",
    feature = "desktop",
    feature = "kittest",
    feature = "terminal"
))]
static ON_CHANGE: RwLock<Option<FontsChanged>> = RwLock::new(None);

#[cfg(feature = "desktop")]
pub(crate) fn on_fonts_changed(notify: impl Fn() + Send + Sync + 'static) {
    if let Ok(mut guard) = ON_CHANGE.write() {
        *guard = Some(Arc::new(notify));
    }
}

#[cfg(any(
    test,
    target_arch = "wasm32",
    feature = "desktop",
    feature = "kittest",
    feature = "terminal"
))]
fn fonts_changed() {
    GENERATION.fetch_add(1, Ordering::Release);
    wake_font_listeners();
}

#[cfg(any(
    test,
    target_arch = "wasm32",
    feature = "desktop",
    feature = "kittest",
    feature = "terminal"
))]
pub(crate) fn wake_font_listeners() {
    let notify = ON_CHANGE.read().ok().and_then(|guard| guard.clone());
    if let Some(notify) = notify {
        notify();
    }
}

#[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
pub fn set_default_fonts(fonts: DefaultFonts) {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::Custom(fonts);
        drop(guard);
        fonts_changed();
    }
}

#[cfg(all(feature = "scene", any(feature = "desktop", feature = "terminal")))]
pub(crate) fn set_fallback_fonts(faces: &[FallbackFace]) {
    if faces.is_empty() {
        return;
    }
    if let Ok(mut guard) = ADDED_FALLBACKS.write() {
        *guard = faces.to_vec();
        drop(guard);
        fonts_changed();
    }
}

#[cfg(feature = "desktop")]
pub(crate) fn use_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write()
        && matches!(*guard, DefaultFontSource::Bundled)
    {
        *guard = DefaultFontSource::System;
        drop(guard);
        fonts_changed();
    }
}

/// Add a fallback face to every text engine.
#[cfg(target_arch = "wasm32")]
pub(crate) fn add_fallback_font(bytes: Vec<u8>, face_index: u32) -> Result<(), crate::Error> {
    let bytes = Arc::new(bytes);
    Font::from_shared(Arc::clone(&bytes), face_index)?;
    let mut guard = ADDED_FALLBACKS
        .write()
        .map_err(|_| crate::Error::fixed(crate::ErrorKind::Font, "invalid font data"))?;
    guard.push((bytes, face_index));
    drop(guard);
    fonts_changed();
    Ok(())
}

pub(crate) fn added_fallback_fonts() -> Vec<Font> {
    ADDED_FALLBACKS.read().map_or_else(
        |_| Vec::new(),
        |guard| {
            guard
                .iter()
                .filter_map(|(bytes, index)| Font::from_shared(Arc::clone(bytes), *index).ok())
                .collect()
        },
    )
}

pub(crate) fn default_font_source() -> DefaultFontSource {
    DEFAULT_FONTS
        .read()
        .map_or_else(|_| DefaultFontSource::Bundled, |guard| guard.clone())
}

pub(crate) fn default_fonts_generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}
