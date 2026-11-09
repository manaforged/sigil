#![allow(unreachable_pub)]

const COLUMN_ROWS: u32 = 204;
const MASK_SIDE: u32 = 128;
const MASKS_PER_ROW: u32 = 4;
const MASK_ROWS: u32 = 2;

pub const BUILTIN_ATLAS_WIDTH: u32 = 600;
pub const BUILTIN_ATLAS_HEIGHT: u32 = COLUMN_ROWS + MASK_SIDE * MASK_ROWS;

#[derive(Clone, Copy)]
pub(crate) enum MaskShape {
    Disc,
    Ring,
    Square,
    Diamond,
    Cross,
}

const MASKS: [MaskShape; 5] = [
    MaskShape::Disc,
    MaskShape::Ring,
    MaskShape::Square,
    MaskShape::Diamond,
    MaskShape::Cross,
];

impl MaskShape {
    fn distance(self, dx: f32, dy: f32) -> f32 {
        let (x, y) = (dx.abs(), dy.abs());
        let reach = x.max(y);
        match self {
            Self::Disc => x.hypot(y) - 52.0,
            Self::Ring => (x.hypot(y) - 40.0).abs() - 12.0,
            Self::Square => reach - 44.0,
            Self::Diamond => (x + y) * std::f32::consts::FRAC_1_SQRT_2 - 40.0,
            Self::Cross => {
                ((x - y).abs() * std::f32::consts::FRAC_1_SQRT_2 - 8.0).max(reach - 44.0)
            }
        }
    }
}

#[cfg(any(test, feature = "kittest"))]
#[allow(
    clippy::cast_precision_loss,
    reason = "atlas coordinates are far below the f32 integer limit"
)]
pub(crate) const fn mask_origin(shape: MaskShape) -> (f32, f32) {
    let index = shape as u32;
    (
        (index % MASKS_PER_ROW * MASK_SIDE) as f32,
        (COLUMN_ROWS + index / MASKS_PER_ROW * MASK_SIDE) as f32,
    )
}

pub fn builtin_atlas_rgba8() -> Vec<u8> {
    let w = BUILTIN_ATLAS_WIDTH as usize;
    let h = BUILTIN_ATLAS_HEIGHT as usize;
    let mut out = vec![0u8; w * h * 4];
    for (pixel_idx, pixel) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let col = pixel_idx % w;
        let row = pixel_idx / w;
        let rgba = if row >= COLUMN_ROWS as usize {
            mask_pixel(col, row - COLUMN_ROWS as usize)
        } else if col < 200 {
            mono_pixel(col, row)
        } else if col < 400 {
            subpixel_pixel(col)
        } else {
            poly_pixel(col, row)
        };
        pixel.copy_from_slice(&rgba);
    }
    out
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn mask_pixel(col: usize, row: usize) -> [u8; 4] {
    let side = MASK_SIDE as usize;
    let index = row / side * MASKS_PER_ROW as usize + col / side;
    let Some(shape) = MASKS
        .get(index)
        .filter(|_| col / side < MASKS_PER_ROW as usize)
    else {
        return [255, 255, 255, 0];
    };
    let centre = |at: usize| (at % side) as f32 + 0.5 - MASK_SIDE as f32 / 2.0;
    let coverage = (0.5 - shape.distance(centre(col), centre(row))).clamp(0.0, 1.0);
    [255, 255, 255, (coverage * 255.0).round() as u8]
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn mono_pixel(col: usize, row: usize) -> [u8; 4] {
    let dx = col as f32 - 100.0;
    let dy = row as f32 - 100.0;
    let dist = dx.hypot(dy);
    let alpha = ((1.0 - (dist / 90.0).clamp(0.0, 1.0)) * 255.0) as u8;
    [255, 255, 255, alpha]
}

const fn subpixel_pixel(col: usize) -> [u8; 4] {
    let alpha = match (col - 200) % 3 {
        0 => 255,
        1 => 0,
        _ => 128,
    };
    [255, 255, 255, alpha]
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn poly_pixel(col: usize, _row: usize) -> [u8; 4] {
    let t = (col - 400) as f32 / 199.0;
    let red = ((1.0 - t) * (1.0 - t) * 255.0) as u8;
    let green = (t.powf(0.4) * 255.0) as u8;
    [red, green, 0, 255]
}
