use resvg::usvg::filter::{self, ColorInterpolation, ColorMatrixKind, Input, Kind, Primitive};
use resvg::usvg::{Group, NonZeroRect, Transform};

use crate::scene::command::Filter;
use crate::scene::shadow::Shadow;
use crate::style::srgb::decode;

use super::fallback::Fallback;

const EPSILON: f32 = 1e-3;
const IDENTITY: [f32; 20] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    0.0,
];

/// The scene filters equal to a group's SVG filters under `t`, or why there are none.
pub(super) fn filters(group: &Group, t: Transform) -> Result<Vec<Filter>, Fallback> {
    let mut out = Vec::new();
    if group.filters().is_empty() {
        return Ok(out);
    }
    let scale = uniform(t).ok_or(Fallback::FilterTransform)?;
    for filter in group.filters() {
        chain(filter, scale, &mut out)?;
    }
    Ok(out)
}

fn chain(filter: &filter::Filter, scale: f32, out: &mut Vec<Filter>) -> Result<(), Fallback> {
    let mut previous: Option<&str> = None;
    for primitive in filter.primitives() {
        if !covers(primitive.rect(), filter.rect()) {
            return Err(Fallback::FilterRegion);
        }
        out.extend(mapped(primitive, previous, scale)?);
        previous = Some(primitive.result());
    }
    Ok(())
}

fn mapped(
    primitive: &Primitive,
    previous: Option<&str>,
    scale: f32,
) -> Result<Option<Filter>, Fallback> {
    let refused = Err(Fallback::Filter(name(primitive.kind())));
    match primitive.kind() {
        Kind::GaussianBlur(blur) if chained(blur.input(), previous) => {
            let (x, y) = (blur.std_dev_x().get(), blur.std_dev_y().get());
            if (x - y).abs() > EPSILON {
                return refused;
            }
            Ok((x > 0.0).then_some(Filter::Blur(x * scale)))
        }
        Kind::DropShadow(shadow) if chained(shadow.input(), previous) => {
            let (across, down) = (shadow.std_dev_x().get(), shadow.std_dev_y().get());
            if (across - down).abs() > EPSILON {
                return refused;
            }
            let color = shadow.color();
            let [red, green, blue] = [color.red, color.green, color.blue].map(decode);
            let offset = [shadow.dx() * scale, shadow.dy() * scale];
            let tint = [red, green, blue, shadow.opacity().get()];
            Ok(Some(Filter::DropShadow(Shadow::new(
                offset,
                across * scale,
                0.0,
                tint,
            ))))
        }
        Kind::ColorMatrix(matrix)
            if chained(matrix.input(), previous)
                && primitive.color_interpolation() == ColorInterpolation::SRGB =>
        {
            match matrix.kind() {
                ColorMatrixKind::Saturate(amount) => Ok(Some(Filter::Saturate(amount.get()))),
                ColorMatrixKind::HueRotate(degrees) => Ok(Some(Filter::HueRotate(*degrees))),
                ColorMatrixKind::Matrix(values) if identity(values) => Ok(None),
                ColorMatrixKind::Matrix(_) | ColorMatrixKind::LuminanceToAlpha => refused,
            }
        }
        _ => refused,
    }
}

fn chained(input: &Input, previous: Option<&str>) -> bool {
    match (input, previous) {
        (Input::SourceGraphic, None) => true,
        (Input::Reference(name), Some(last)) => name == last,
        _ => false,
    }
}

fn identity(values: &[f32]) -> bool {
    values.len() == IDENTITY.len()
        && values
            .iter()
            .zip(IDENTITY)
            .all(|(value, unit)| (value - unit).abs() < EPSILON)
}

fn covers(inner: NonZeroRect, outer: NonZeroRect) -> bool {
    inner.left() <= outer.left() + EPSILON
        && inner.top() <= outer.top() + EPSILON
        && inner.right() >= outer.right() - EPSILON
        && inner.bottom() >= outer.bottom() - EPSILON
}

fn uniform(t: Transform) -> Option<f32> {
    let flat = t.kx.abs() < EPSILON && t.ky.abs() < EPSILON;
    let even = (t.sx - t.sy).abs() <= EPSILON * t.sx.abs().max(t.sy.abs());
    (flat && even && t.sx > 0.0).then_some(t.sx)
}

const fn name(kind: &Kind) -> &'static str {
    match kind {
        Kind::Blend(_) => "feBlend",
        Kind::ColorMatrix(_) => "feColorMatrix",
        Kind::ComponentTransfer(_) => "feComponentTransfer",
        Kind::Composite(_) => "feComposite",
        Kind::ConvolveMatrix(_) => "feConvolveMatrix",
        Kind::DiffuseLighting(_) => "feDiffuseLighting",
        Kind::DisplacementMap(_) => "feDisplacementMap",
        Kind::DropShadow(_) => "feDropShadow",
        Kind::Flood(_) => "feFlood",
        _ => source_name(kind),
    }
}

const fn source_name(kind: &Kind) -> &'static str {
    match kind {
        Kind::GaussianBlur(_) => "feGaussianBlur",
        Kind::Image(_) => "feImage",
        Kind::Merge(_) => "feMerge",
        Kind::Morphology(_) => "feMorphology",
        Kind::Offset(_) => "feOffset",
        Kind::SpecularLighting(_) => "feSpecularLighting",
        Kind::Tile(_) => "feTile",
        _ => "feTurbulence",
    }
}
