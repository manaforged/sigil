use super::PathCommand;
use crate::scene::command::PathPaint;
use crate::style::Color;

pub(super) fn mix(hash: &mut u64, byte: u8) {
    *hash = crate::fnv::mix(*hash, u64::from(byte));
}

pub(super) fn mix_f32(hash: &mut u64, value: f32) {
    *hash = crate::fnv::bytes(*hash, &value.to_bits().to_le_bytes());
}

fn mix_rgba(hash: &mut u64, color: Color) {
    *hash = crate::fnv::mix(*hash, crate::fnv::rgba(color));
}

pub(super) fn hash_cmd(hash: &mut u64, cmd: PathCommand) {
    *hash = crate::scene::command::hash_path_cmd(*hash, cmd);
}

pub(super) fn mix_stroke(hash: &mut u64, stroke: Option<(Color, f32)>) {
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
