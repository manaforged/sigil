//! CPU hinted glyph rasterization — the Wave-3 small-text path.
//!
//! Slug's analytic vector coverage is resolution-independent but cannot
//! grid-fit stems the way a hinted CPU raster can at small sizes. This
//! module rasterizes a glyph at its PHYSICAL pixel size through skrifa's
//! hinting engine into an 8-bit alpha mask (zeno fill), for upload into
//! an R8 atlas. Vector Slug remains the path for large, animated, or
//! transformed text; selection happens at draw time (Wave-3 D2).
//!
//! Subpixel positioning: `bin` selects one of [`SUBPIXEL_BINS`] fixed
//! x-offsets (`bin / SUBPIXEL_BINS` px) baked into the mask, so a run
//! can place glyphs at fractional advances without re-rasterizing per
//! position.

use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, HintingInstance, HintingOptions, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Command, Mask, Origin, Vector};

/// Number of subpixel x-offset bins baked into raster masks.
pub(crate) const SUBPIXEL_BINS: u8 = 4;

/// Logical-px size at or below which the raster path is preferred over
/// vector Slug (the plan's ~20 px small-text threshold).
pub(crate) const RASTER_MAX_LOGICAL_PX: f32 = 20.0;

/// One rasterized glyph: 8-bit coverage rows, top-left placement
/// relative to the pen position on the baseline.
pub(crate) struct GlyphMask {
    /// Coverage bytes, row-major, `width * height` entries.
    pub data: Vec<u8>,
    /// Mask width in physical pixels.
    pub width: u32,
    /// Mask height in physical pixels.
    pub height: u32,
    /// Horizontal offset from the pen position to the mask's left edge.
    pub left: i32,
    /// Vertical offset from the BASELINE up to the mask's top edge
    /// (positive above the baseline, matching font y-up conventions).
    pub top: i32,
}

/// Cache key for one rasterized mask. Physical pixel size is quantized
/// to bits so `f32` keys hash exactly; `bin` is the subpixel x bin.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct RasterKey {
    /// Identity of the font data (engine-assigned).
    pub font_id: u64,
    /// Face index within a TTC.
    pub font_index: u32,
    /// Glyph id within the face.
    pub glyph_id: u32,
    /// Physical pixel size (`size_px * scale`), bit-exact.
    pub physical_px_bits: u32,
    /// Normalized variable-font location in axis order.
    pub variation_coords: Box<[i16]>,
    /// Subpixel x-offset bin, `0..SUBPIXEL_BINS`.
    pub bin: u8,
}

/// Font-instance identity needed to resolve a variable glyph outline.
#[derive(Clone, Copy)]
pub(crate) struct RasterFont<'a> {
    pub bytes: &'a [u8],
    pub index: u32,
    pub variation_coords: &'a [i16],
}

/// Rasterize `glyph_id` at `physical_px` (logical size × DPI scale)
/// with hinting, at subpixel x-offset `bin / SUBPIXEL_BINS` px.
///
/// Returns `None` for glyphs without an outline (spaces, controls),
/// zero-coverage results, or unparseable font data — the draw path
/// falls back to Slug for those.
pub(crate) fn rasterize_glyph(
    font: RasterFont<'_>,
    glyph_id: u32,
    physical_px: f32,
    bin: u8,
) -> Option<GlyphMask> {
    let font_ref = FontRef::from_index(font.bytes, font.index).ok()?;
    let outlines = font_ref.outline_glyphs();
    let glyph = outlines.get(GlyphId::new(glyph_id))?;
    let size = Size::new(physical_px.max(1.0));
    let coords: Vec<NormalizedCoord> = font
        .variation_coords
        .iter()
        .copied()
        .map(NormalizedCoord::from_bits)
        .collect();
    let location = LocationRef::new(&coords);

    // Hinted draw; a face the hinter rejects falls back to unhinted
    // rather than losing the glyph.
    let mut pen = ZenoPen::new(f32::from(bin % SUBPIXEL_BINS) / f32::from(SUBPIXEL_BINS));
    let hinter = HintingInstance::new(&outlines, size, location, HintingOptions::default());
    let drawn = match &hinter {
        Ok(instance) => glyph.draw(DrawSettings::hinted(instance, false), &mut pen),
        Err(_) => glyph.draw(DrawSettings::unhinted(size, location), &mut pen),
    };
    drawn.ok()?;
    if pen.commands.is_empty() {
        return None;
    }

    // BottomLeft origin: zeno interprets the path y-up (font convention)
    // and reports `top` as the rise above the baseline; rows still come
    // back top-down, ready for image upload. The default TopLeft origin
    // would read the y-up outline as y-down and flip every glyph.
    //
    // The no-op `.inspect()` is load-bearing: `render()` computes the
    // BottomLeft `top` from the mask's cached height, which is only
    // populated by `ensure_size()` — reachable publicly via `inspect`.
    // Without it, `top` comes back as the glyph's min-y instead of its
    // max-y (zeno 0.3.3).
    let (data, placement) = Mask::new(&pen.commands)
        .origin(Origin::BottomLeft)
        .inspect(|_, _, _| {})
        .render();
    if placement.width == 0 || placement.height == 0 || data.iter().all(|&b| b == 0) {
        return None;
    }
    Some(GlyphMask {
        data,
        width: placement.width,
        height: placement.height,
        left: placement.left,
        top: placement.top,
    })
}

/// skrifa pen sink producing zeno path commands. Applies the subpixel
/// x offset at record time so the mask bakes the shifted coverage.
struct ZenoPen {
    commands: Vec<Command>,
    dx: f32,
}

impl ZenoPen {
    fn new(dx: f32) -> Self {
        Self {
            commands: Vec::with_capacity(32),
            dx,
        }
    }

    fn v(&self, x: f32, y: f32) -> Vector {
        Vector::new(x + self.dx, y)
    }
}

impl OutlinePen for ZenoPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(Command::MoveTo(self.v(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(Command::LineTo(self.v(x, y)));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.commands
            .push(Command::QuadTo(self.v(cx, cy), self.v(x, y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.commands.push(Command::CurveTo(
            self.v(cx0, cy0),
            self.v(cx1, cy1),
            self.v(x, y),
        ));
    }

    fn close(&mut self) {
        self.commands.push(Command::Close);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    const FIRA: &[u8] = include_bytes!("../../text/assets/fonts/FiraSans-Regular.ttf");

    fn glyph_a(font: &FontRef<'_>) -> u32 {
        font.charmap().map('A').unwrap().to_u32()
    }

    #[test]
    fn small_glyph_has_ink_and_sane_dims() {
        let font = FontRef::new(FIRA).unwrap();
        let gid = glyph_a(&font);
        let mask = rasterize_glyph(test_font(), gid, 12.0, 0).unwrap();
        assert!(mask.width > 0 && mask.width <= 16, "width {}", mask.width);
        assert!(
            mask.height > 0 && mask.height <= 16,
            "height {}",
            mask.height
        );
        assert_eq!(mask.data.len(), (mask.width * mask.height) as usize);
        assert!(mask.data.iter().any(|&b| b > 128), "expected solid ink");
        // 'A' sits on the baseline: top ≈ cap height above it.
        assert!(
            mask.top > 0,
            "cap-height glyph rises above baseline, got top {}",
            mask.top
        );
    }

    /// 'L' is bottom-heavy (the foot stroke). If the mask came back
    /// vertically flipped, the top row would carry the heavy ink.
    #[test]
    fn mask_is_not_vertically_flipped() {
        let font = FontRef::new(FIRA).unwrap();
        let gid = font.charmap().map('L').unwrap().to_u32();
        let mask = rasterize_glyph(test_font(), gid, 16.0, 0).unwrap();
        let w = mask.width as usize;
        let row_ink = |r: usize| -> u32 {
            mask.data[r * w..(r + 1) * w]
                .iter()
                .map(|&b| u32::from(b))
                .sum()
        };
        let top = row_ink(1);
        let bottom = row_ink(mask.height as usize - 2);
        assert!(
            bottom > top,
            "'L' foot must be at the bottom row (top ink {top}, bottom ink {bottom})"
        );
    }

    #[test]
    fn subpixel_bins_shift_coverage() {
        let font = FontRef::new(FIRA).unwrap();
        let gid = glyph_a(&font);
        let bin0 = rasterize_glyph(test_font(), gid, 12.0, 0).unwrap();
        let bin2 = rasterize_glyph(test_font(), gid, 12.0, 2).unwrap();
        // A half-pixel shift must change the coverage distribution.
        assert!(
            bin0.width != bin2.width || bin0.data != bin2.data,
            "bin 0 and bin 2 rendered identical masks"
        );
    }

    #[test]
    fn physical_size_scales_mask() {
        let font = FontRef::new(FIRA).unwrap();
        let gid = glyph_a(&font);
        let s100 = rasterize_glyph(test_font(), gid, 12.0, 0).unwrap();
        let s200 = rasterize_glyph(test_font(), gid, 24.0, 0).unwrap();
        let ratio = f64::from(s200.height) / f64::from(s100.height);
        assert!(
            (1.6..=2.4).contains(&ratio),
            "24px mask should be ~2x the 12px mask, got {ratio}"
        );
    }

    #[test]
    fn outline_less_glyph_returns_none() {
        let font = FontRef::new(FIRA).unwrap();
        let space = font.charmap().map(' ').unwrap().to_u32();
        assert!(rasterize_glyph(test_font(), space, 12.0, 0).is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn variable_weight_changes_mask_and_cache_identity() {
        use skrifa::MetadataProvider as _;

        let bytes = std::fs::read("/System/Library/Fonts/SFNSMono.ttf")
            .expect("macOS system mono variable font");
        let font = FontRef::new(&bytes).expect("system mono parses");
        let gid = glyph_a(&font);
        let regular_location = font.axes().location([("wght", 400.0)]);
        let bold_location = font.axes().location([("wght", 700.0)]);
        let regular: Box<[i16]> = regular_location
            .coords()
            .iter()
            .map(|coord| coord.to_bits())
            .collect();
        let bold: Box<[i16]> = bold_location
            .coords()
            .iter()
            .map(|coord| coord.to_bits())
            .collect();
        let regular_mask = rasterize_glyph(
            RasterFont {
                bytes: &bytes,
                index: 0,
                variation_coords: &regular,
            },
            gid,
            16.0,
            0,
        )
        .expect("regular mask");
        let bold_mask = rasterize_glyph(
            RasterFont {
                bytes: &bytes,
                index: 0,
                variation_coords: &bold,
            },
            gid,
            16.0,
            0,
        )
        .expect("bold mask");

        assert_ne!(regular, bold);
        assert!(
            regular_mask.width != bold_mask.width || regular_mask.data != bold_mask.data,
            "font weight must change the rasterized glyph mask",
        );
    }

    const fn test_font() -> RasterFont<'static> {
        RasterFont {
            bytes: FIRA,
            index: 0,
            variation_coords: &[],
        }
    }
}
