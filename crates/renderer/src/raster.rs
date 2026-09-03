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

// Hygiene exception: until Wave-3 D2 wires draw-time path selection and
// the R8 atlas, everything here is exercised only by this module's
// tests — the lib graph sees it as dead. Same staged-landing pattern as
// the G1.1 shaper skeleton.
#![allow(dead_code)]

use std::collections::HashMap;

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{
    DrawSettings, HintingInstance, HintingOptions, OutlineGlyphCollection, OutlinePen,
};
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct RasterKey {
    /// Identity of the font data (engine-assigned).
    pub font_id: u64,
    /// Face index within a TTC.
    pub font_index: u32,
    /// Glyph id within the face.
    pub glyph_id: u32,
    /// Physical pixel size (`size_px * scale`), bit-exact.
    pub physical_px_bits: u32,
    /// Subpixel x-offset bin, `0..SUBPIXEL_BINS`.
    pub bin: u8,
}

/// Cache key for one hinting instance: face identity plus the physical
/// pixel size it was configured for (hinting options are always default).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct HintKey {
    /// Identity of the font data (engine-assigned).
    pub font_id: u64,
    /// Face index within a TTC.
    pub font_index: u32,
    /// Physical pixel size (`size_px * scale`), bit-exact.
    pub physical_px_bits: u32,
}

/// Maximum live hinting instances before an arbitrary one is evicted.
const HINTER_LIMIT: usize = 48;

/// Hinting instances kept across calls, one per face and pixel size.
#[derive(Default)]
pub(crate) struct Hinters {
    map: HashMap<HintKey, Option<HintingInstance>>,
}

impl Hinters {
    /// Return the instance for `key`, running the font's hinting program
    /// only on a miss. `None` means the face rejects hinting.
    fn get(
        &mut self,
        key: HintKey,
        outlines: &OutlineGlyphCollection<'_>,
        size: Size,
    ) -> Option<&HintingInstance> {
        if self.map.len() >= HINTER_LIMIT
            && !self.map.contains_key(&key)
            && let Some(&stale) = self.map.keys().next()
        {
            self.map.remove(&stale);
        }
        self.map
            .entry(key)
            .or_insert_with(|| {
                HintingInstance::new(
                    outlines,
                    size,
                    LocationRef::default(),
                    HintingOptions::default(),
                )
                .ok()
            })
            .as_ref()
    }

    /// Rasterize `key.glyph_id` at `key`'s physical pixel size (logical size × DPI scale)
    /// with hinting, at subpixel x-offset `bin / SUBPIXEL_BINS` px.
    ///
    /// Returns `None` for glyphs without an outline (spaces, controls),
    /// zero-coverage results, or unparseable font data — the draw path
    /// falls back to Slug for those.
    pub(crate) fn rasterize(&mut self, font_bytes: &[u8], key: &RasterKey) -> Option<GlyphMask> {
        let font = FontRef::from_index(font_bytes, key.font_index).ok()?;
        let outlines = font.outline_glyphs();
        let glyph = outlines.get(GlyphId::new(key.glyph_id))?;
        let physical_px = f32::from_bits(key.physical_px_bits);
        let size = Size::new(physical_px.max(1.0));

        // Hinted draw; a face the hinter rejects falls back to unhinted
        // rather than losing the glyph.
        let mut pen = ZenoPen::new(f32::from(key.bin % SUBPIXEL_BINS) / f32::from(SUBPIXEL_BINS));
        let hint = HintKey {
            font_id: key.font_id,
            font_index: key.font_index,
            physical_px_bits: key.physical_px_bits,
        };
        let drawn = match self.get(hint, &outlines, size) {
            Some(instance) => glyph.draw(DrawSettings::hinted(instance, false), &mut pen),
            None => glyph.draw(
                DrawSettings::unhinted(size, LocationRef::default()),
                &mut pen,
            ),
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
#[path = "raster_tests.rs"]
mod tests;
