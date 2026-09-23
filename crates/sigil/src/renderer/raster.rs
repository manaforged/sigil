use std::collections::HashMap;

use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{
    DrawSettings, HintingInstance, HintingOptions, OutlineGlyphCollection, OutlinePen,
};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Command, Mask, Origin, Vector};

pub(crate) const SUBPIXEL_BINS: u8 = 4;

pub(crate) const RASTER_MAX_LOGICAL_PX: f32 = 20.0;

pub(crate) struct GlyphMask {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct RasterKey {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    pub physical_px_bits: u32,
    pub bin: u8,
    pub coords: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct HintKey {
    pub font_id: u64,
    pub font_index: u32,
    pub physical_px_bits: u32,
    pub coords: u64,
}

const HINTER_LIMIT: usize = 48;

#[derive(Default)]
pub(crate) struct Hinters {
    map: HashMap<HintKey, Option<HintingInstance>>,
}

impl Hinters {
    fn get(
        &mut self,
        key: HintKey,
        outlines: &OutlineGlyphCollection<'_>,
        size: Size,
        coords: &[NormalizedCoord],
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
                    LocationRef::new(coords),
                    HintingOptions::default(),
                )
                .ok()
            })
            .as_ref()
    }

    pub(crate) fn rasterize(
        &mut self,
        font_bytes: &[u8],
        key: &RasterKey,
        variation_coords: &[i16],
    ) -> Option<GlyphMask> {
        let font = FontRef::from_index(font_bytes, key.font_index).ok()?;
        let outlines = font.outline_glyphs();
        let glyph = outlines.get(GlyphId::new(key.glyph_id))?;
        let physical_px = f32::from_bits(key.physical_px_bits);
        let size = Size::new(physical_px.max(1.0));
        let coords: Vec<NormalizedCoord> = variation_coords
            .iter()
            .map(|&c| NormalizedCoord::from_bits(c))
            .collect();

        let mut pen = ZenoPen::new(f32::from(key.bin % SUBPIXEL_BINS) / f32::from(SUBPIXEL_BINS));
        let hint = HintKey {
            font_id: key.font_id,
            font_index: key.font_index,
            physical_px_bits: key.physical_px_bits,
            coords: key.coords,
        };
        let drawn = match self.get(hint, &outlines, size, &coords) {
            Some(instance) => glyph.draw(DrawSettings::hinted(instance, false), &mut pen),
            None => glyph.draw(
                DrawSettings::unhinted(size, LocationRef::new(&coords)),
                &mut pen,
            ),
        };
        drawn.ok()?;
        if pen.commands.is_empty() {
            return None;
        }

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
