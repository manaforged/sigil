use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(target_os = "macos")]
use skrifa::string::StringId;
#[cfg(target_os = "macos")]
use skrifa::{FontRef, MetadataProvider};
use zed_font_kit::family_name::FamilyName;
use zed_font_kit::font::Font as PlatformFont;
use zed_font_kit::handle::Handle;
use zed_font_kit::properties::{Properties, Stretch, Style};
use zed_font_kit::source::SystemSource;

use super::SystemFonts;
use crate::text::font_family::FontFamily;
use crate::text::shaper::{Font, Script};

pub(super) fn load() -> SystemFonts {
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

pub(super) fn script_faces(script: Script) -> Vec<Font> {
    let source = SystemSource::new();
    let mut cache = FontCache::default();
    crate::text::script_fonts::candidates(script)
        .iter()
        .filter_map(|name| {
            let family = [FamilyName::Title((*name).to_owned())];
            load_family(&source, &family, &mut cache).map(|family| family.select(400).clone())
        })
        .collect()
}

fn load_family(
    source: &SystemSource,
    candidates: &[FamilyName],
    cache: &mut FontCache,
) -> Option<FontFamily> {
    for candidate in candidates {
        if let FamilyName::Title(name) = candidate {
            let Ok(family) = source.select_family_by_name(name) else {
                continue;
            };
            let mut faces = Vec::new();
            for handle in family.fonts() {
                if let Some(face) = load_family_face(source, handle.clone(), cache) {
                    if face.1.has_weight_axis() {
                        return Some(FontFamily::new(face.1));
                    }
                    faces.push(face);
                }
            }
            if let Some(resolved) = FontFamily::from_weighted(faces) {
                return Some(resolved);
            }
        } else if let Some(face) = select_generic_face(source, candidate, cache) {
            return Some(FontFamily::new(face));
        }
    }
    None
}

fn load_family_face(
    source: &SystemSource,
    handle: Handle,
    cache: &mut FontCache,
) -> Option<(u16, Font)> {
    let platform_font = PlatformFont::from_handle(&handle).ok()?;
    let properties = platform_font.properties();
    if properties.style != Style::Normal || properties.stretch != Stretch::NORMAL {
        return None;
    }
    let actual_weight = platform_weight(&platform_font);
    load_platform_font(source, handle, &platform_font, cache).map(|font| (actual_weight, font))
}

fn select_generic_face(
    source: &SystemSource,
    candidate: &FamilyName,
    cache: &mut FontCache,
) -> Option<Font> {
    let handle = source
        .select_best_match(core::slice::from_ref(candidate), &Properties::new())
        .ok()?;
    let platform_font = PlatformFont::from_handle(&handle).ok()?;
    load_platform_font(source, handle, &platform_font, cache)
}

fn load_platform_font(
    source: &SystemSource,
    handle: Handle,
    font: &PlatformFont,
    cache: &mut FontCache,
) -> Option<Font> {
    #[cfg(target_os = "macos")]
    if let Handle::Native { .. } = &handle {
        let native = handle.native_as::<core_text::font::CTFont>()?;
        let path = native.copy_descriptor().font_path()?;
        return cache.font_from_path_name(path, &font.postscript_name()?);
    }
    let portable = portable_handle(source, handle, font)?;
    cache.font(portable)
}

fn portable_handle(source: &SystemSource, handle: Handle, font: &PlatformFont) -> Option<Handle> {
    if matches!(handle, Handle::Native { .. }) {
        let name = font.postscript_name()?;
        source.select_by_postscript_name(&name).ok()
    } else {
        Some(handle)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn platform_weight(font: &PlatformFont) -> u16 {
    font.properties().weight.0.clamp(1.0, 1000.0).round() as u16
}

#[derive(Default)]
struct FontCache {
    blobs: HashMap<u64, Vec<Arc<Vec<u8>>>>,
    paths: HashMap<PathBuf, Arc<Vec<u8>>>,
    fonts: Vec<CachedFont>,
}

struct CachedFont {
    bytes: Arc<Vec<u8>>,
    index: u32,
    font: Font,
}

impl FontCache {
    fn font(&mut self, handle: Handle) -> Option<Font> {
        let (bytes, index) = match handle {
            Handle::Path { path, font_index } => (self.path_bytes(path)?, font_index),
            Handle::Memory { bytes, font_index } => (bytes, font_index),
            Handle::Native { .. } => return None,
        };
        let bytes = self.intern(bytes);
        if let Some(cached) = self
            .fonts
            .iter()
            .find(|font| font.index == index && Arc::ptr_eq(&font.bytes, &bytes))
        {
            return Some(cached.font.clone());
        }
        let font = Font::from_shared(Arc::clone(&bytes), index).ok()?;
        self.fonts.push(CachedFont {
            bytes,
            index,
            font: font.clone(),
        });
        Some(font)
    }

    fn intern(&mut self, bytes: Arc<Vec<u8>>) -> Arc<Vec<u8>> {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let bucket = self.blobs.entry(hasher.finish()).or_default();
        if let Some(shared) = bucket
            .iter()
            .find(|shared| shared.as_slice() == bytes.as_slice())
        {
            return Arc::clone(shared);
        }
        bucket.push(Arc::clone(&bytes));
        bytes
    }

    fn path_bytes(&mut self, path: PathBuf) -> Option<Arc<Vec<u8>>> {
        if let Some(bytes) = self.paths.get(&path) {
            return Some(Arc::clone(bytes));
        }
        let bytes = Arc::new(fs::read(&path).ok()?);
        self.paths.insert(path, Arc::clone(&bytes));
        Some(bytes)
    }

    #[cfg(target_os = "macos")]
    fn font_from_path_name(&mut self, path: PathBuf, postscript: &str) -> Option<Font> {
        let bytes = self.path_bytes(path)?;
        for index in 0..128 {
            let Ok(candidate) = FontRef::from_index(&bytes, index) else {
                break;
            };
            let matches = candidate
                .localized_strings(StringId::POSTSCRIPT_NAME)
                .any(|name| name.to_string() == postscript);
            if matches {
                return Font::from_shared(bytes, index).ok();
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use core_text::font::{CTFontUIFontType, new_ui_font_for_language};

    use super::{FontCache, FontFamily};

    pub(super) fn ui_family(
        font_type: CTFontUIFontType,
        cache: &mut FontCache,
    ) -> Option<FontFamily> {
        let font = new_ui_font_for_language(font_type, 0.0, None);
        let path = font.copy_descriptor().font_path()?;
        let postscript = font.postscript_name();
        cache
            .font_from_path_name(path, &postscript)
            .map(FontFamily::new)
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
