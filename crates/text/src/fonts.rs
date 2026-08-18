//! Process-global font source for every [`OwnedTextEngine`].
//!
//! Sigil bundles `FiraSans` + `CascadiaMono` so a freshly launched app always
//! renders *something*. A consumer that wants different faces app-wide (e.g.
//! the host app rendering Geist) registers them here BEFORE any engine is
//! constructed. Both the layout-measure engine and the paint-time
//! `shape_glyph_run` engine read this override in
//! [`FontCollection::with_default`](crate::FontCollection), so one call
//! repoints text everywhere and keeps measurement and rendering on the same
//! face. Register once during startup, before `sigil::launch`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Bootstrap face override: raw sfnt (TTF/OTF) bytes + face index per family.
///
/// A `None` slot keeps Sigil's bundled default for that family, so a consumer
/// can override only the sans face and inherit the bundled monospace, or both.
/// `Clone` is cheap — the bytes are reference-counted via [`Arc`]. `Default` is
/// all-`None` (every family keeps Sigil's bundled face), so consumers can spread
/// `..DefaultFonts::default()` and set only the slots they override.
#[derive(Clone, Default)]
pub struct DefaultFonts {
    /// Sans / primary face — `(sfnt_bytes, face_index)`. `face_index` is `0`
    /// for single-face files.
    pub sans: Option<(Arc<Vec<u8>>, u32)>,
    /// Monospace face — `(sfnt_bytes, face_index)`.
    pub mono: Option<(Arc<Vec<u8>>, u32)>,
    /// Icon face selected by `font-icon` / `font_family = "icon"` — an icon font
    /// (e.g. codicon) whose glyphs live in the Private Use Area. `None` leaves
    /// `select("icon")` falling back to the sans face. Defaults to `None`.
    pub icon: Option<(Arc<Vec<u8>>, u32)>,
    /// Display face selected by `font-display` / `font_family = "display"` —
    /// wordmarks and titles (e.g. Bebas Neue). `None` falls back to sans.
    pub display: Option<(Arc<Vec<u8>>, u32)>,
}

#[derive(Clone, Default)]
pub(crate) enum DefaultFontSource {
    #[default]
    Bundled,
    Custom(DefaultFonts),
    System,
}

static DEFAULT_FONTS: RwLock<DefaultFontSource> = RwLock::new(DefaultFontSource::Bundled);
/// Bumped on every [`set_default_fonts`]; a live [`OwnedTextEngine`](crate::OwnedTextEngine)
/// compares it against the generation it last shaped with to know when to rebuild.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Override the bootstrap sans/mono faces for every text engine.
///
/// Unlike a one-shot registration this is **re-settable at runtime**: each call
/// replaces the faces and bumps a generation counter. Live engines notice the bump
/// on their next shaping call, rebuild their font collection, and clear their
/// paragraph cache — so a debug panel can A/B fonts against the running window.
pub fn set_default_fonts(fonts: DefaultFonts) {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::Custom(fonts);
        GENERATION.fetch_add(1, Ordering::Release);
    }
}

/// Use the native desktop UI families selected by CoreText, DirectWrite, or Fontconfig.
///
/// This replaces a prior custom override. Call before constructing a text engine.
pub fn use_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write() {
        *guard = DefaultFontSource::System;
        GENERATION.fetch_add(1, Ordering::Release);
    }
}

/// Select system fonts only while the caller has not installed custom defaults.
#[doc(hidden)]
pub fn prefer_system_fonts() {
    if let Ok(mut guard) = DEFAULT_FONTS.write()
        && matches!(*guard, DefaultFontSource::Bundled)
    {
        *guard = DefaultFontSource::System;
        GENERATION.fetch_add(1, Ordering::Release);
    }
}

/// Clone the selected source without holding its read lock during font parsing.
pub(crate) fn default_font_source() -> DefaultFontSource {
    DEFAULT_FONTS
        .read()
        .map_or_else(|_| DefaultFontSource::Bundled, |guard| guard.clone())
}

/// The current override generation. Changes whenever [`set_default_fonts`] is called.
pub(crate) fn default_fonts_generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}
