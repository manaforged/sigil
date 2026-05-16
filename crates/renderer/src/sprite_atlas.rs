//! Built-in showcase atlas generator for Phase R7 sprite split.

#![allow(unreachable_pub)]
//!
//! Generates the 600x200 RGBA8 texture used by `scenes::sprite()` without
//! requiring caller-side GPU access at scene-build time. The atlas is uploaded
//! lazily by [`crate::sprite_paint::SpritePipeline`] on the first frame that
//! references [`crate::sprite_paint::BUILTIN_SPRITE_ATLAS_ID`].
//!
//! # Layout
//!
//! | x range  | Region    | Content |
//! |----------|-----------|---------|
//! | 0–199    | Mono      | Soft disk; RGB = white, A = soft coverage. |
//! | 200–399  | Subpixel  | Repeating R/G/B vertical stripes (3 px period). |
//! | 400–599  | Poly      | Horizontal red→green gradient, vertical alpha falloff. |

/// Width of the built-in showcase atlas in texels (3 regions × 200 px).
pub const BUILTIN_ATLAS_WIDTH: u32 = 600;
/// Height of the built-in showcase atlas in texels.
pub const BUILTIN_ATLAS_HEIGHT: u32 = 200;

/// Generate the raw RGBA8 bytes for the built-in 600×200 showcase atlas.
pub fn builtin_atlas_rgba8() -> Vec<u8> {
    let w = BUILTIN_ATLAS_WIDTH as usize;
    let h = BUILTIN_ATLAS_HEIGHT as usize;
    let mut out = vec![0u8; w * h * 4];
    for (pixel_idx, pixel) in out.chunks_exact_mut(4).enumerate() {
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

/// Soft-disk pixel for the Mono region.
///
/// RGB is white so the Mono shader composites cleanly via `color * coverage`.
/// Alpha encodes a smooth disk with radius 90 px centred at texel (100, 100).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn mono_pixel(col: usize, row: usize) -> [u8; 4] {
    let dx = col as f32 - 100.0;
    let dy = row as f32 - 100.0;
    let dist = dx.hypot(dy);
    let alpha = ((1.0 - (dist / 90.0).clamp(0.0, 1.0)) * 255.0) as u8;
    [255, 255, 255, alpha]
}

/// Repeating R/G/B stripe for the Subpixel region (3 px period, a = 220).
const fn subpixel_pixel(col: usize) -> [u8; 4] {
    let alpha = match (col - 200) % 3 {
        0 => 255,
        1 => 0,
        _ => 128,
    };
    [255, 255, 255, alpha]
}

/// Horizontal red→green gradient + vertical alpha falloff for the Poly region.
///
/// The tint supplied by the scene is ignored in the Poly shader branch, so the
/// atlas content must carry the full color signal. Green dominates at x=500
/// (the horizontal centre of the Poly region) and alpha peaks at row 100.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn poly_pixel(col: usize, _row: usize) -> [u8; 4] {
    let t = (col - 400) as f32 / 199.0;
    let red = ((1.0 - t) * (1.0 - t) * 255.0) as u8;
    let green = (t.powf(0.4) * 255.0) as u8;
    [red, green, 0, 255]
}
