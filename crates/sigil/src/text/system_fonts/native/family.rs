use zed_font_kit::family_name::FamilyName;
use zed_font_kit::source::SystemSource;

use super::super::SystemFonts;
use super::{FontCache, load_family};

pub(in crate::text::system_fonts) fn load() -> SystemFonts {
    let source = SystemSource::new();
    let mut cache = FontCache::default();
    let sans = sans_candidates();
    let mono = mono_candidates();
    let serif = serif_candidates();
    #[cfg(target_os = "macos")]
    let system_sans = macos::ui_family(core_text::font::kCTFontSystemFontType, &mut cache);
    let mono = load_family(&source, &mono, &mut cache);
    let serif = load_family(&source, &serif, &mut cache);
    SystemFonts {
        #[cfg(target_os = "macos")]
        sans: system_sans.or_else(|| load_family(&source, &sans, &mut cache)),
        #[cfg(not(target_os = "macos"))]
        sans: load_family(&source, &sans, &mut cache),
        mono,
        serif,
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use core_text::font::{CTFontUIFontType, new_ui_font_for_language};

    use super::super::{FontCache, FontFaces};

    pub(super) fn ui_family(
        font_type: CTFontUIFontType,
        cache: &mut FontCache,
    ) -> Option<FontFaces> {
        let font = new_ui_font_for_language(font_type, 0.0, None);
        let path = font.copy_descriptor().font_path()?;
        let postscript = font.postscript_name();
        cache
            .font_from_path_name(path, &postscript)
            .map(FontFaces::new)
    }
}

#[cfg(target_os = "macos")]
fn sans_candidates() -> Vec<FamilyName> {
    titles(&["Helvetica Neue", "Helvetica"], FamilyName::SansSerif)
}

#[cfg(target_family = "windows")]
fn sans_candidates() -> Vec<FamilyName> {
    titles(&["Segoe UI Variable", "Segoe UI"], FamilyName::SansSerif)
}

#[cfg(not(any(target_os = "macos", target_family = "windows")))]
fn sans_candidates() -> Vec<FamilyName> {
    titles(
        &["Inter", "Noto Sans", "DejaVu Sans"],
        FamilyName::SansSerif,
    )
}

#[cfg(target_os = "macos")]
fn mono_candidates() -> Vec<FamilyName> {
    titles(&[".SF NS Mono", "Menlo"], FamilyName::Monospace)
}

#[cfg(target_family = "windows")]
fn mono_candidates() -> Vec<FamilyName> {
    titles(&["Cascadia Mono", "Consolas"], FamilyName::Monospace)
}

#[cfg(not(any(target_os = "macos", target_family = "windows")))]
fn mono_candidates() -> Vec<FamilyName> {
    titles(
        &["JetBrains Mono", "Noto Sans Mono", "DejaVu Sans Mono"],
        FamilyName::Monospace,
    )
}

#[cfg(target_os = "macos")]
fn serif_candidates() -> Vec<FamilyName> {
    titles(&["New York", "Times New Roman"], FamilyName::Serif)
}

#[cfg(target_family = "windows")]
fn serif_candidates() -> Vec<FamilyName> {
    titles(&["Georgia", "Times New Roman"], FamilyName::Serif)
}

#[cfg(not(any(target_os = "macos", target_family = "windows")))]
fn serif_candidates() -> Vec<FamilyName> {
    titles(&["Noto Serif", "DejaVu Serif"], FamilyName::Serif)
}

fn titles(names: &[&str], generic: FamilyName) -> Vec<FamilyName> {
    names
        .iter()
        .map(|name| FamilyName::Title((*name).to_owned()))
        .chain(core::iter::once(generic))
        .collect()
}
