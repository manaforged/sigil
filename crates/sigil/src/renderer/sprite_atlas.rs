#![allow(unreachable_pub)]

pub const BUILTIN_ATLAS_WIDTH: u32 = 600;
pub const BUILTIN_ATLAS_HEIGHT: u32 = 200;

pub fn builtin_atlas_rgba8() -> Vec<u8> {
    let w = BUILTIN_ATLAS_WIDTH as usize;
    let h = BUILTIN_ATLAS_HEIGHT as usize;
    let mut out = vec![0u8; w * h * 4];
    for (pixel_idx, pixel) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let col = pixel_idx % w;
        let row = pixel_idx / w;
        let rgba = if col < 200 {
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
