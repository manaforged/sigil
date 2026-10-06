#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

use crate::text::font_family::FontFamily;
use crate::text::shaper::{Font, Script};

#[derive(Clone)]
pub(crate) struct SystemFonts {
    pub(crate) sans: Option<FontFamily>,
    pub(crate) mono: Option<FontFamily>,
    pub(crate) serif: Option<FontFamily>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load() -> SystemFonts {
    static SYSTEM_FONTS: OnceLock<SystemFonts> = OnceLock::new();
    SYSTEM_FONTS.get_or_init(native::load).clone()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn script_fallbacks(script: Script) -> Vec<Font> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static SCRIPTS: Mutex<Option<HashMap<Script, Vec<Font>>>> = Mutex::new(None);
    let mut guard = SCRIPTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard
        .get_or_insert_with(HashMap::new)
        .entry(script)
        .or_insert_with(|| native::script_faces(script))
        .clone()
}

#[cfg(target_arch = "wasm32")]
pub(crate) const fn load() -> SystemFonts {
    SystemFonts {
        sans: None,
        mono: None,
        serif: None,
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn script_fallbacks(script: Script) -> Vec<Font> {
    web::request(script);
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(all(test, target_os = "macos"))]
#[path = "system_fonts_tests.rs"]
mod tests;
