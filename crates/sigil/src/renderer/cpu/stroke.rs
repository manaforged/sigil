#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use tiny_skia::{FillRule, Mask, Path, PathBuilder, Pixmap, Point as TsPoint};

use crate::renderer::scene_command::{Stroke, StrokeBand, StrokeDash};
use crate::scene::corner_radii::CornerRadii;

use super::bound::{Bound, expand, rounded_rect};
use super::shapes::{Brush, solid};

pub(super) fn stroke(
    target: &mut Pixmap,
    rect: Rect,
    radii: CornerRadii,
    stroke: Stroke,
    brush: &Brush<'_>,
) {
    let Some(band) = StrokeBand::of(stroke, rect) else {
        return;
    };
    let Some(path) = ring(target, rect, radii, band, brush) else {
        return;
    };
    let paint = solid(stroke.color, brush.opacity);
    let clip = brush.mask_for(&paint);
    let dashed = match band
        .dash
        .map(|dash| dash_mask(target, rect, dash, brush, clip))
    {
        Some(None) => return,
        found => found.flatten(),
    };
    target.fill_path(
        &path,
        &paint,
        FillRule::EvenOdd,
        brush.transform,
        dashed.as_ref().or(clip),
    );
}

fn ring(
    target: &Pixmap,
    rect: Rect,
    radii: CornerRadii,
    band: StrokeBand,
    brush: &Brush<'_>,
) -> Option<Path> {
    let base = radii.normalized_for(rect.width, rect.height);
    let bound = Bound::of(target, brush.transform);
    let outer = rounded_rect(
        expand(rect, band.outside),
        CornerRadii(base.map(|r| r + band.outside)),
        bound,
    )?;
    let inner = rounded_rect(
        expand(rect, -band.inside),
        CornerRadii(base.map(|r| (r - band.inside).max(0.0))),
        bound,
    );
    let mut builder = PathBuilder::new();
    builder.push_path(&outer);
    if let Some(inner) = inner {
        builder.push_path(&inner);
    }
    builder.finish()
}

fn dash_mask(
    target: &Pixmap,
    rect: Rect,
    dash: StrokeDash,
    brush: &Brush<'_>,
    clip: Option<&Mask>,
) -> Option<Mask> {
    let inverse = brush.transform.invert()?;
    let (width, height) = (target.width(), target.height());
    let mut mask = Mask::new(width, height)?;
    let center = [
        f64::from(rect.width).mul_add(0.5, f64::from(rect.origin.x)),
        f64::from(rect.height).mul_add(0.5, f64::from(rect.origin.y)),
    ];
    let row = width as usize;
    let clip_data = clip.map(Mask::data);
    for (index, value) in mask.data_mut().iter_mut().enumerate() {
        let mut p = [TsPoint::from_xy(
            (index % row) as f32 + 0.5,
            (index / row) as f32 + 0.5,
        )];
        inverse.map_points(&mut p);
        let [local] = p;
        let on = dash.covers(Point::new(local.x, local.y), center);
        let limit = clip_data.map_or(u8::MAX, |data| data.get(index).copied().unwrap_or(0));
        *value = if on { limit } else { 0 };
    }
    Some(mask)
}
