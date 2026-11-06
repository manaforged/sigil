use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use ahash::AHashMap;

use crate::hal::{ParagraphMetrics, TextEngine};
use crate::style::{StyledTextEngine, TextLineHeight, TextStyle};
use unicode_segmentation::UnicodeSegmentation;

use crate::bidi;
use crate::text::FaceSpec;
use crate::text::cache::ParagraphCache;
use crate::text::paragraph::{FaceTable, Paragraph, TextMetrics, face_table};
#[cfg(test)]
use crate::text::shaper::Script;
use crate::text::shaper::{FaceStyle, FeatureTag, Font, Shaper};
use smallvec::SmallVec;

mod lines;
mod shape;
mod spans;

use lines::{LineFit, LineStrut, break_lines};
use shape::char_script;
#[cfg(any(test, feature = "kittest"))]
use shape::script_name;
#[cfg(test)]
use shape::{PushRunArgs, push_run, run_has_disallowed_notdef, shape_validated};
#[cfg(any(test, feature = "kittest"))]
use spans::shape_segments;
use spans::{shape_spans, style_spans};

pub struct OwnedTextEngine {
    fonts: RefCell<FontCollection>,
    font_gen: Cell<u64>,
    font_epoch: Cell<u64>,
    #[cfg(any(test, feature = "kittest"))]
    style: TextStyle,
    cache: RefCell<ParagraphCache>,
    chains: RefCell<AHashMap<ChainKey, Arc<[Font]>>>,
    face_tables: RefCell<AHashMap<FaceKey, FaceTable>>,
}

type FaceKey = (u32, SmallVec<[(u64, u16, u32); 8]>);

const FACE_TABLES_CAP: usize = 1_024;

type ChainKey = (u64, u16, ChainScripts);

impl OwnedTextEngine {
    #[must_use]
    pub fn new(style: TextStyle) -> Self {
        #[cfg(not(any(test, feature = "kittest")))]
        drop(style);
        Self {
            fonts: RefCell::new(FontCollection::with_default()),
            font_gen: Cell::new(crate::text::fonts::default_fonts_generation()),
            font_epoch: Cell::new(0),
            #[cfg(any(test, feature = "kittest"))]
            style,
            cache: RefCell::new(ParagraphCache::new()),
            chains: RefCell::new(AHashMap::new()),
            face_tables: RefCell::new(AHashMap::new()),
        }
    }

    #[must_use]
    pub fn thread() -> Rc<Self> {
        thread_local! {
            static ENGINE: Rc<OwnedTextEngine> =
                Rc::new(OwnedTextEngine::new(TextStyle {
                    font_size: 16.0,
                    ..TextStyle::default()
                }));
        }
        ENGINE.with(Rc::clone)
    }

    fn refresh_fonts(&self) {
        let generation = crate::text::fonts::default_fonts_generation();
        if generation != self.font_gen.get() {
            *self.fonts.borrow_mut() = FontCollection::with_default();
            self.font_gen.set(generation);
            self.fonts_changed();
        }
    }

    fn fonts_changed(&self) {
        self.font_epoch.set(self.font_epoch.get().wrapping_add(1));
        self.cache.borrow_mut().clear();
        self.chains.borrow_mut().clear();
        self.face_tables.borrow_mut().clear();
    }

    #[must_use]
    pub fn font_generation(&self) -> u64 {
        self.refresh_fonts();
        self.font_epoch.get()
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    pub fn layout_paragraph_ellipsized(
        &self,
        text: &str,
        max_width: f32,
        style: &TextStyle,
    ) -> Result<Paragraph, crate::Error> {
        const ELLIPSIS: &str = "\u{2026}";
        let full = self.layout_paragraph_styled(text, None, style)?;
        if full.width() <= max_width {
            return Ok(full);
        }
        let ellipsis = self.layout_paragraph_styled(ELLIPSIS, None, style)?;
        if ellipsis.width() >= max_width {
            return Ok(ellipsis);
        }
        let budget = max_width - ellipsis.width();

        let bounds: Vec<usize> = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        let bound_at = |k: usize| bounds.get(k).copied().unwrap_or(text.len());
        let (mut lo, mut hi) = (0usize, bounds.len() - 1);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            let prefix = text.get(..bound_at(mid)).unwrap_or("");
            let fits = self
                .layout_paragraph_styled(prefix, None, style)
                .is_ok_and(|p| p.width() <= budget);
            if fits {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let kept = text.get(..bound_at(lo)).unwrap_or("").trim_end();
        let mut out = String::with_capacity(kept.len() + ELLIPSIS.len());
        out.push_str(kept);
        out.push_str(ELLIPSIS);
        self.layout_paragraph_styled(&out, None, style)
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn fonts(&self) -> std::cell::Ref<'_, FontCollection> {
        self.fonts.borrow()
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn debug_segments_for_tests(&self, text: &str) -> Vec<(u32, u32, u8, &'static str)> {
        let fonts = self.fonts.borrow();
        let primary = fonts.select_weight("sans", self.style.font_weight);
        let chain = fonts.fallback_chain(primary, self.style.font_weight, text);
        let style = to_shaper_style(&self.style);
        let paragraph_bidi = bidi::ParagraphBidi::new(text);
        shape_segments(&chain, text, &style, &paragraph_bidi)
            .into_iter()
            .map(|s| (s.start, s.end, s.level, script_name(s.script)))
            .collect()
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn register_font(&mut self, bytes: Vec<u8>, face_index: u32) -> Result<(), crate::Error> {
        self.fonts.borrow_mut().register(bytes, face_index)?;
        self.fonts_changed();
        crate::text::fonts::wake_font_listeners();
        Ok(())
    }

    #[cfg(any(test, feature = "kittest"))]
    pub fn register_font_family(
        &mut self,
        family: &str,
        bytes: Vec<u8>,
        face_index: u32,
    ) -> Result<(), crate::Error> {
        self.fonts
            .borrow_mut()
            .register_family(family, bytes, face_index)?;
        self.fonts_changed();
        crate::text::fonts::wake_font_listeners();
        Ok(())
    }

    fn chain(
        &self,
        fonts: &FontCollection,
        primary: &Font,
        weight: u16,
        scripts: &ChainScripts,
    ) -> Arc<[Font]> {
        let key = (primary.id(), weight, scripts.clone());
        if let Some(hit) = self.chains.borrow().get(&key) {
            return Arc::clone(hit);
        }
        let chain: Arc<[Font]> = fonts.chain_for(primary, weight, scripts).into();
        self.chains.borrow_mut().insert(key, Arc::clone(&chain));
        chain
    }

    fn face_table(&self, faces: &[FaceSpec], size_px: f32) -> FaceTable {
        let key: FaceKey = (
            size_px.to_bits(),
            faces
                .iter()
                .map(|face| (face.font.id(), face.weight, face.skew.to_bits()))
                .collect(),
        );
        if let Some(hit) = self.face_tables.borrow().get(&key) {
            return std::sync::Arc::clone(hit);
        }
        let table = face_table(faces, size_px);
        let mut tables = self.face_tables.borrow_mut();
        if tables.len() >= FACE_TABLES_CAP {
            tables.clear();
        }
        tables.insert(key, std::sync::Arc::clone(&table));
        table
    }

    fn cached_paragraph(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Option<Paragraph> {
        let cache = self.cache.borrow();
        if let Some((cached, _)) = cache.get(text, max_width, style) {
            return Some(cached.clone());
        }
        let limit = max_width?;
        cache
            .get(text, None, style)
            .filter(|(_, width)| {
                LineFit {
                    wrapped: false,
                    width: *width,
                }
                .fits(limit)
            })
            .map(|(unbounded, _)| unbounded.clone())
    }

    fn build_paragraph(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> (Paragraph, LineFit) {
        let fonts = self.fonts.borrow();
        let primary = fonts.select_weight(&style.font_family, style.font_weight);
        let shaper = Shaper::new(primary);
        let shaper_style = to_shaper_style(style);
        let strut = LineStrut::from_shaper(&shaper, &shaper_style, resolve_line_height(style));
        let paragraph_bidi = bidi::ParagraphBidi::new(text);
        let scripts = chain_scripts(text);
        let mut chain_of = |face: &Font, weight: u16| self.chain(&fonts, face, weight, &scripts);
        let (spans, faces) = style_spans(&fonts, text, style, &shaper_style, &mut chain_of);
        let runs = shape_spans(&spans, text, &paragraph_bidi);
        let min_content_hint = runs
            .iter()
            .filter(|s| !s.is_break_opportunity)
            .map(|s| s.advance)
            .fold(0.0_f32, f32::max);
        let (lines, fit) = break_lines(&runs, max_width, strut, &paragraph_bidi);
        let paragraph = Paragraph::new(
            lines,
            TextMetrics {
                font_size: shaper_style.size_px,
                line_height: strut.line_height,
            },
            self.face_table(&faces, shaper_style.size_px),
            min_content_hint,
        );
        (paragraph, fit)
    }
}

impl TextEngine for OwnedTextEngine {
    type Error = crate::Error;
    type Paragraph = Paragraph;

    #[cfg(any(test, feature = "kittest"))]
    fn layout_paragraph(
        &self,
        text: &str,
        max_width: Option<f32>,
    ) -> Result<Paragraph, crate::Error> {
        let style = self.style.clone();
        self.layout_paragraph_styled(text, max_width, &style)
    }

    #[cfg(any(test, feature = "kittest"))]
    fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
    }
}

impl StyledTextEngine for OwnedTextEngine {
    fn layout_paragraph_styled(
        &self,
        text: &str,
        max_width: Option<f32>,
        style: &TextStyle,
    ) -> Result<Paragraph, crate::Error> {
        self.refresh_fonts();
        if let Some(cached) = self.cached_paragraph(text, max_width, style) {
            return Ok(cached);
        }
        let (para, fit) = self.build_paragraph(text, max_width, style);
        let key = if fit.wrapped { max_width } else { None };
        self.cache
            .borrow_mut()
            .insert(text, key, style, (para.clone(), fit.width));
        Ok(para)
    }

    fn font_generation(&self) -> u64 {
        Self::font_generation(self)
    }
}

fn to_shaper_style(style: &TextStyle) -> FaceStyle {
    FaceStyle {
        size_px: font_size_px(style),
        weight: style.font_weight,
        features: if style.tabular_nums {
            smallvec::smallvec![FeatureTag(*b"tnum")]
        } else {
            smallvec::SmallVec::new()
        },
        letter_spacing_em: finite_or(style.letter_spacing_em, 0.0),
        cell_grid: style
            .runs
            .as_ref()
            .is_some_and(|runs| runs.0.iter().any(|run| run.face.square)),
    }
}

fn font_size_px(style: &TextStyle) -> f32 {
    finite_or(style.font_size, TextStyle::default().font_size).max(1.0)
}

fn resolve_line_height(style: &TextStyle) -> f32 {
    let size = font_size_px(style);
    let line = |height: TextLineHeight| match height {
        TextLineHeight::FontSizeRelative(factor) => size * factor.max(0.1),
        TextLineHeight::Absolute(value) => value,
    };
    finite_or(line(style.line_height), line(TextLineHeight::default())).max(1.0)
}

const fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
#[path = "engine/tests.rs"]
mod tests;

mod collection;
pub use collection::FontCollection;
use collection::{ChainScripts, chain_scripts};
