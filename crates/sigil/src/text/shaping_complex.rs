#![allow(clippy::redundant_pub_crate)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use harfrust::{
    Direction, Feature, FontRef as HrFontRef, GlyphBuffer, Script as HrScript, ShapeOptions,
    ShapePlan, ShapePlanKey, ShaperInstance, Tag, UnicodeBuffer, script as hr_script,
};
use skrifa::FontRef;
use skrifa::MetadataProvider;
use skrifa::instance::{Location, LocationRef, Size};
use smallvec::SmallVec;

use crate::text::glyph::{GlyphId, GlyphPos, ShapedRun};
use crate::text::shaper::{FaceStyle, Font, Script, TextSegment, classify_segment_script};

pub(crate) fn shape(
    segment: &TextSegment<'_>,
    style: &FaceStyle,
    font: &Font,
) -> Option<ShapedRun> {
    if segment.text.is_empty() {
        return Some(ShapedRun::empty());
    }
    if let Some(run) = crate::text::numeric_run::shape(segment, style, font, shape_full) {
        return Some(run);
    }
    shape_full(segment, style, font)
}

pub(crate) fn shape_full(
    segment: &TextSegment<'_>,
    style: &FaceStyle,
    font: &Font,
) -> Option<ShapedRun> {
    if segment.text.is_empty() {
        return Some(ShapedRun::empty());
    }
    let bytes = font.bytes_arc();
    let hr_font = HrFontRef::from_index(bytes.as_slice(), font.face_index()).ok()?;
    let location = style.location(font);
    let instance = ShaperInstance::from_coords(&hr_font, location.coords().iter().copied());
    let hr_shaper = font
        .tables(&hr_font)
        .shaper(&hr_font)
        .instance(Some(&instance))
        .build();

    let script = map_script(classify_segment_script(segment));
    let direction = direction_of(segment.level);

    let mut buffer = SCRATCH.with_borrow_mut(Option::take).unwrap_or_default();
    buffer.push_str(segment.text);
    buffer.set_direction(direction);
    buffer.set_script(script);

    let features = build_features(style);
    let plan = plan(
        &hr_shaper,
        (font.id(), font.face_index()),
        (script, direction),
        (&instance, features.as_slice()),
    );
    let options = ShapeOptions::new()
        .plan(Some(&plan))
        .point_size(Some(style.size_px))
        .features(&features);
    let glyphs_buf = hr_shaper.shape(buffer, options);
    if glyphs_buf.is_empty() {
        recycle(glyphs_buf);
        return None;
    }

    let run = scaled_run(&glyphs_buf, bytes.as_slice(), style, (font, &location));
    recycle(glyphs_buf);
    run
}

fn scaled_run(
    glyphs: &GlyphBuffer,
    bytes: &[u8],
    style: &FaceStyle,
    (font, location): (&Font, &Location),
) -> Option<ShapedRun> {
    let skrifa_font = FontRef::from_index(bytes, font.face_index()).ok()?;
    let size = Size::new(style.size_px.max(1.0));
    let face_metrics = skrifa_font.metrics(size, LocationRef::new(location.coords()));
    let upem = f32::from(face_metrics.units_per_em);
    let scale = if upem > 0.0 {
        style.size_px / upem
    } else {
        1.0
    };
    Some(build_glyph_run(glyphs, scale, &face_metrics))
}

const fn direction_of(level: u8) -> Direction {
    if level & 1 == 1 {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    }
}

fn recycle(glyphs: GlyphBuffer) {
    SCRATCH.with_borrow_mut(|slot| *slot = Some(glyphs.clear()));
}

const PLANS_PER_FACE: usize = 16;

type PlanFace = (u64, u32);

thread_local! {
    static SCRATCH: RefCell<Option<UnicodeBuffer>> = const { RefCell::new(None) };
    static PLANS: RefCell<HashMap<PlanFace, Vec<Rc<ShapePlan>>>> = RefCell::new(HashMap::new());
}

fn plan(
    shaper: &harfrust::Shaper<'_>,
    face: PlanFace,
    (script, direction): (HrScript, Direction),
    (instance, features): (&ShaperInstance, &[Feature]),
) -> Rc<ShapePlan> {
    let key = ShapePlanKey::new(Some(script), direction)
        .instance(Some(instance))
        .features(features);
    PLANS.with_borrow_mut(|plans| {
        let known = plans.entry(face).or_default();
        if let Some(plan) = known.iter().find(|plan| key.matches(plan)) {
            return Rc::clone(plan);
        }
        let plan = Rc::new(ShapePlan::new(
            shaper,
            direction,
            Some(script),
            None,
            features,
        ));
        if known.len() >= PLANS_PER_FACE {
            known.remove(0);
        }
        known.push(Rc::clone(&plan));
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

fn build_features(style: &FaceStyle) -> SmallVec<[harfrust::Feature; 4]> {
    style
        .font
        .features
        .iter()
        .map(|(tag, value)| Feature::new(Tag::new(&tag), u32::from(value), ..))
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
) -> ShapedRun {
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
    let width = max_x - min_x;
    ShapedRun {
        glyphs: out,
        baseline,
        width,
    }
}

#[cfg(test)]
#[path = "shaping_complex/tests.rs"]
mod tests;
