use super::PathCmd;
use crate::scene::command::PathPaint;
use crate::style::Rgba8;

pub(super) fn mix(hash: &mut u64, byte: u8) {
    *hash = crate::fnv::mix(*hash, u64::from(byte));
}

pub(super) fn mix_f32(hash: &mut u64, value: f32) {
    for byte in value.to_bits().to_le_bytes() {
        mix(hash, byte);
    }
}

fn mix_rgba(hash: &mut u64, color: Rgba8) {
    for byte in [color.r, color.g, color.b, color.a] {
        mix(hash, byte);
    }
}

fn mix_tagged(hash: &mut u64, tag: u8, values: &[f32]) {
    mix(hash, tag);
    for value in values {
        mix_f32(hash, *value);
    }
}

pub(super) fn hash_cmd(hash: &mut u64, cmd: PathCmd) {
    match cmd {
        PathCmd::MoveTo(x, y) => mix_tagged(hash, 1, &[x, y]),
        PathCmd::LineTo(x, y) => mix_tagged(hash, 2, &[x, y]),
        PathCmd::QuadTo(cx, cy, x, y) => mix_tagged(hash, 3, &[cx, cy, x, y]),
        PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
            mix_tagged(hash, 4, &[c1x, c1y, c2x, c2y, x, y]);
        }
        PathCmd::Close => mix(hash, 5),
    }
}

pub(super) fn mix_stroke(hash: &mut u64, stroke: Option<(Rgba8, f32)>) {
    match stroke {
        Some((color, width)) => {
            mix(hash, 1);
            mix_rgba(hash, color);
            mix_f32(hash, width);
        }
        None => mix(hash, 0),
    }
}

pub(super) fn mix_fill(hash: &mut u64, fill: Option<&PathPaint>) {
    match fill {
        Some(PathPaint::Solid(color)) => {
            mix(hash, 1);
            mix_rgba(hash, *color);
        }
        Some(PathPaint::Gradient(gradient)) => *hash = gradient.key(crate::fnv::mix(*hash, 2)),
        None => mix(hash, 0),
    }
}
