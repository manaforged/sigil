use crate::geometry::{Point, Rect};
use crate::renderer::raster::RASTER_MAX_LOGICAL_PX;
use crate::renderer::scene_command::state::invert;
use crate::renderer::scene_command::walk::{IDENTITY, transform_rect};
use crate::renderer::scene_command::{GlyphRun, Transform};
use crate::renderer::slug::text_tuning;
use crate::scene::command::effects::transform_point;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlyphMode {
    Mask,
    Outline,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphPlan {
    pub(crate) mode: GlyphMode,
    pub(crate) bounds: Rect,
}

pub(crate) fn run_ink(run: &GlyphRun) -> Option<Rect> {
    let origin = run.rect.origin;
    let ink = run.paragraph()?.ink_rect()?;
    Some(Rect::new(
        Point::new(origin.x + ink.origin.x, origin.y + ink.origin.y),
        ink.width,
        ink.height,
    ))
}

pub(crate) fn plan_glyph_run(
    run: &GlyphRun,
    transform: Transform,
    scale: f32,
) -> Option<GlyphPlan> {
    invert(transform)?;
    if run.positioned_glyphs().is_empty() {
        return None;
    }
    let ink = run_ink(run)?.expand(1.0 / scale.max(f32::MIN_POSITIVE));
    let mode = if text_tuning().raster
        && transform == IDENTITY
        && run.style.font_size <= RASTER_MAX_LOGICAL_PX
    {
        GlyphMode::Mask
    } else {
        GlyphMode::Outline
    };
    Some(GlyphPlan {
        mode,
        bounds: transform_rect(transform, ink),
    })
}

pub(crate) fn axis_aligned(transform: Transform) -> bool {
    let [[_, b, _], [c, _, _]] = transform;
    b.abs() < f32::MIN_POSITIVE && c.abs() < f32::MIN_POSITIVE
}

pub(crate) fn pen_shift(transform: Transform, pen: Point, scale: f32) -> Point {
    let tuning = text_tuning();
    let [[xx, _, _], [_, yy, _]] = transform;
    if !(tuning.enabled && tuning.snap && axis_aligned(transform))
        || scale <= 0.0
        || xx.abs() < f32::MIN_POSITIVE
        || yy.abs() < f32::MIN_POSITIVE
    {
        return Point::new(0.0, 0.0);
    }
    let mapped = transform_point(transform, pen);
    let snap = |value: f32| (value * scale).round() / scale - value;
    Point::new(snap(mapped.x) / xx, snap(mapped.y) / yy)
}

pub(crate) fn text_gamma(linear: [f32; 4], encoded: bool) -> f32 {
    let tuning = text_tuning();
    if !tuning.enabled {
        return 1.0;
    }
    let [r, g, b, _] = linear;
    let lum = 0.0722f32.mul_add(b, 0.7152f32.mul_add(g, 0.2126 * r));
    let perceptual = if encoded {
        crate::style::srgb::encode(lum)
    } else {
        lum
    };
    if perceptual >= 0.5 {
        tuning.gamma_light_on_dark
    } else {
        tuning.gamma_dark_on_light
    }
}

pub(crate) fn coverage_curve(coverage: f32, gamma: f32) -> f32 {
    let coverage = coverage.clamp(0.0, 1.0);
    if gamma <= 0.0 || coverage <= 0.0 {
        return coverage;
    }
    coverage.powf(gamma.recip())
}
