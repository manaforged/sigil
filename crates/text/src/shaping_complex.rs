#![allow(clippy::redundant_pub_crate)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use harfrust::{
    Direction, Feature, FontRef as HrFontRef, GlyphBuffer, Script as HrScript, ShapeOptions,
    ShapePlan, ShaperInstance, Tag, UnicodeBuffer, Variation, script as hr_script,
};
use skrifa::FontRef;
use skrifa::MetadataProvider;
use skrifa::instance::{LocationRef, Size};
use smallvec::SmallVec;

use crate::glyph::{GlyphId, GlyphPos, GlyphRun, Rect};
use crate::shaper::{Font, Script, Style, TextSegment, classify_segment_script};

pub(crate) fn shape(segment: &TextSegment<'_>, style: &Style, font: &Font) -> Option<GlyphRun> {
    if segment.text.is_empty() {
        return Some(GlyphRun::empty());
    }
    if let Some(run) = crate::numeric_run::shape(segment, style, font, shape_full) {
        return Some(run);
    }
    shape_full(segment, style, font)
}

pub(crate) fn shape_full(
    segment: &TextSegment<'_>,
    style: &Style,
    font: &Font,
) -> Option<GlyphRun> {
    if segment.text.is_empty() {
        return Some(GlyphRun::empty());
    }
    let bytes = font.bytes_arc();
    let hr_font = HrFontRef::from_index(bytes.as_slice(), font.face_index()).ok()?;
    let instance = shaper_instance(&hr_font, style, font);
    let hr_shaper = font
        .tables(&hr_font)
        .shaper(&hr_font)
        .instance(Some(&instance))
        .build();

    let script = classify_segment_script(segment);
    let hr_script = map_script(script);
    let direction = if segment.level & 1 == 1 {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    };

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(segment.text);
    buffer.set_direction(direction);
    buffer.set_script(hr_script);

    let features = build_features(style);
    let key = Key {
        id: font.id(),
        index: font.face_index(),
        script: hr_script,
        direction,
        features: features.iter().map(|feature| feature.tag).collect(),
    };
    let plan = plan(&hr_shaper, &key, &features);
    let options = ShapeOptions::new()
        .plan(Some(&plan))
        .point_size(Some(style.size_px))
        .features(&features);
    let glyphs_buf = hr_shaper.shape(buffer, options);
    if glyphs_buf.is_empty() {
        return None;
    }

    let skrifa_font = FontRef::from_index(bytes.as_slice(), font.face_index()).ok()?;
    let size = Size::new(style.size_px.max(1.0));
    let location = font.location(style.weight, style.size_px);
    let face_metrics = skrifa_font.metrics(size, LocationRef::new(location.coords()));
    let upem = f32::from(face_metrics.units_per_em);
    let scale = if upem > 0.0 {
        style.size_px / upem
    } else {
        1.0
    };

    Some(build_glyph_run(&glyphs_buf, scale, &face_metrics))
}

fn shaper_instance(font_ref: &HrFontRef<'_>, style: &Style, font: &Font) -> ShaperInstance {
    let settings = font.variation_settings(style.weight, style.size_px);
    ShaperInstance::from_variations(
        font_ref,
        settings.iter().map(|(tag, value)| Variation {
            tag: Tag::new(tag),
            value: *value,
        }),
    )
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    id: u64,
    index: u32,
    script: HrScript,
    direction: Direction,
    features: SmallVec<[Tag; 4]>,
}

thread_local! {
    static PLANS: RefCell<HashMap<Key, Rc<ShapePlan>>> = RefCell::new(HashMap::new());
}

fn plan(shaper: &harfrust::Shaper<'_>, key: &Key, features: &[Feature]) -> Rc<ShapePlan> {
    PLANS.with_borrow_mut(|plans| {
        if let Some(plan) = plans.get(key) {
            return Rc::clone(plan);
        }
        let plan = Rc::new(ShapePlan::new(
            shaper,
            key.direction,
            Some(key.script),
            None,
            features,
        ));
        plans.insert(key.clone(), Rc::clone(&plan));
        plan
    })
}

const fn map_script(script: Script) -> HrScript {
    match script {
        Script::Latin => hr_script::LATIN,
        Script::Cjk => hr_script::HAN,
        Script::Arabic => hr_script::ARABIC,
        Script::Hebrew => hr_script::HEBREW,
        Script::Devanagari => hr_script::DEVANAGARI,
        Script::Khmer => hr_script::KHMER,
        Script::Thai => hr_script::THAI,
        Script::Lao => hr_script::LAO,
        Script::Common | Script::Other => hr_script::COMMON,
    }
}

fn build_features(style: &Style) -> SmallVec<[harfrust::Feature; 4]> {
    style
        .features
        .iter()
        .map(|tag| harfrust::Feature::new(harfrust::Tag::new(&tag.0), 1, ..))
        .collect()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn build_glyph_run(
    glyphs: &GlyphBuffer,
    scale: f32,
    face_metrics: &skrifa::metrics::Metrics,
) -> GlyphRun {
    let infos = glyphs.glyph_infos();
    let positions = glyphs.glyph_positions();
    let mut out: SmallVec<[GlyphPos; 16]> = SmallVec::with_capacity(infos.len());
    let mut pen_x: f32 = 0.0;
    let mut min_x: f32 = 0.0;
    let mut max_x: f32 = 0.0;
    for (info, pos) in infos.iter().zip(positions.iter()) {
        let advance = pos.x_advance as f32 * scale;
        let offset_x = pos.x_offset as f32 * scale;
        let offset_y = pos.y_offset as f32 * scale;
        let id_u16 = u16::try_from(info.glyph_id).unwrap_or(0);
        out.push(GlyphPos {
            id: GlyphId(id_u16),
            advance,
            offset_x,
            offset_y,
            cluster: info.cluster,
        });
        pen_x += advance;
        if pen_x > max_x {
            max_x = pen_x;
        }
        if pen_x < min_x {
            min_x = pen_x;
        }
    }
    let baseline = face_metrics.ascent;
    let bbox = Rect {
        x: min_x,
        y: -face_metrics.ascent,
        w: max_x - min_x,
        h: face_metrics.ascent - face_metrics.descent,
    };
    GlyphRun {
        glyphs: out,
        baseline,
        bbox,
    }
}

#[cfg(test)]
#[path = "shaping_complex/tests.rs"]
mod tests;
