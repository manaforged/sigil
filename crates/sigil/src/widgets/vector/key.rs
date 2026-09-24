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
    mix(hash, color.r);
    mix(hash, color.g);
    mix(hash, color.b);
    mix(hash, color.a);
}

fn mix_stops(hash: &mut u64, stops: &[(f32, Rgba8)]) {
    for &(pos, color) in stops {
        mix_f32(hash, pos);
        mix_rgba(hash, color);
    }
}

pub(super) fn hash_cmd(hash: &mut u64, cmd: PathCmd) {
    match cmd {
        PathCmd::MoveTo(x, y) => {
            mix(hash, 1);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::LineTo(x, y) => {
            mix(hash, 2);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::QuadTo(cx, cy, x, y) => {
            mix(hash, 3);
            mix_f32(hash, cx);
            mix_f32(hash, cy);
            mix_f32(hash, x);
            mix_f32(hash, y);
        }
        PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
            mix(hash, 4);
            mix_f32(hash, c1x);
            mix_f32(hash, c1y);
            mix_f32(hash, c2x);
            mix_f32(hash, c2y);
            mix_f32(hash, x);
            mix_f32(hash, y);
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
            mix(hash, 2);
            mix_f32(hash, start.0);
            mix_f32(hash, start.1);
            mix_f32(hash, end.0);
            mix_f32(hash, end.1);
            mix_stops(hash, stops);
        }
        Some(PathFill::Radial {
            center,
            radius,
            stops,
        }) => {
            mix(hash, 3);
            mix_f32(hash, center.0);
            mix_f32(hash, center.1);
            mix_f32(hash, *radius);
            mix_stops(hash, stops);
        }
        Some(PathFill::Theme(_)) | None => mix(hash, 0),
    }
}
