use super::{PathCmd, PathFill};
use crate::style::Rgba8;

pub(super) const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

pub(super) fn mix(hash: &mut u64, byte: u8) {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    *hash ^= u64::from(byte);
    *hash = hash.wrapping_mul(PRIME);
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

fn mix_stops(hash: &mut u64, stops: &[(f32, Rgba8)]) {
    for &(pos, color) in stops {
        mix_f32(hash, pos);
        mix_rgba(hash, color);
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

pub(super) fn mix_fill(hash: &mut u64, fill: Option<&PathFill>) {
    match fill {
        Some(PathFill::Solid(color)) => {
            mix(hash, 1);
            mix_rgba(hash, *color);
        }
        Some(PathFill::Linear { start, end, stops }) => {
            mix_tagged(hash, 2, &[start.0, start.1, end.0, end.1]);
            mix_stops(hash, stops);
        }
        Some(PathFill::Radial {
            center,
            radius,
            stops,
        }) => {
            mix_tagged(hash, 3, &[center.0, center.1, *radius]);
            mix_stops(hash, stops);
        }
        Some(PathFill::Theme(_)) | None => mix(hash, 0),
    }
}
