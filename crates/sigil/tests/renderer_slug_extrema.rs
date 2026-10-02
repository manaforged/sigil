#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use std::sync::Arc;

use image::RgbaImage;
use sigil::app::{DefaultFonts, TextTuning, set_default_fonts, set_text_tuning};
use sigil::prelude::*;
use sigil_kittest::Harness;
use sigil_kittest::gpu::{render_scene_to_image, shared_device};

const FONT: &[u8] = include_bytes!("fixtures/text/Barlow-Regular-subset.ttf");
const OFFSETS: u16 = 8;
const SIZES: std::ops::Range<i16> = 12..19;
const SCALES: [u32; 6] = [20, 22, 24, 25, 26, 30];
const VIEWPORT: (u32, u32) = (120, 1500);
const MIN_RING_PX: u32 = 14;

fn lines() -> NodeBuilder {
    let mut column = container().layout(LayoutStyle::flex_column());
    for size in SIZES {
        for step in 0..OFFSETS {
            let lead = f32::from(step) / f32::from(OFFSETS);
            column = column
                .child(container().layout(LayoutStyle::block().fixed_size(1.0, lead)))
                .child(text("o o o").font_size(size));
        }
    }
    column
}

struct Ink {
    width: u32,
    height: u32,
    mask: Vec<bool>,
}

impl Ink {
    fn new(img: &RgbaImage) -> Self {
        let peak = img.pixels().map(|pixel| pixel.0[0]).max().unwrap_or(0);
        let mask = img
            .pixels()
            .map(|pixel| u16::from(pixel.0[0]) * 10 > u16::from(peak) * 3)
            .collect();
        Self {
            width: img.width(),
            height: img.height(),
            mask,
        }
    }

    fn at(&self, x: u32, y: u32) -> bool {
        self.index(x, y)
            .and_then(|index| self.mask.get(index))
            .copied()
            .unwrap_or(false)
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| usize::try_from(y * self.width + x).ok())
            .flatten()
    }

    fn glyph_boxes(&self) -> Vec<[u32; 4]> {
        let mut seen = vec![false; self.mask.len()];
        let mut boxes = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(bounds) = self.flood(&mut seen, x, y) {
                    boxes.push(bounds);
                }
            }
        }
        boxes
    }

    fn flood(&self, seen: &mut [bool], x: u32, y: u32) -> Option<[u32; 4]> {
        if !self.visit(seen, x, y) {
            return None;
        }
        let mut stack = vec![(x, y)];
        let mut bounds = [x, y, x, y];
        while let Some((px, py)) = stack.pop() {
            bounds = [
                bounds[0].min(px),
                bounds[1].min(py),
                bounds[2].max(px),
                bounds[3].max(py),
            ];
            for (nx, ny) in [
                (px.wrapping_sub(1), py),
                (px + 1, py),
                (px, py.wrapping_sub(1)),
                (px, py + 1),
            ] {
                if self.visit(seen, nx, ny) {
                    stack.push((nx, ny));
                }
            }
        }
        Some(bounds)
    }

    fn visit(&self, seen: &mut [bool], x: u32, y: u32) -> bool {
        let Some(slot) = self.index(x, y).and_then(|index| seen.get_mut(index)) else {
            return false;
        };
        if *slot || !self.at(x, y) {
            return false;
        }
        *slot = true;
        true
    }

    fn counter_inked(&self, [left, top, right, bottom]: [u32; 4]) -> bool {
        let span = bottom - top;
        let column = left.midpoint(right);
        (top + span * 3 / 10..bottom - span * 3 / 10).any(|y| self.at(column, y))
    }
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn outline_glyph_counters_stay_open_at_curve_joins() {
    let Some((device, queue)) = shared_device().unwrap() else {
        return;
    };
    set_default_fonts(DefaultFonts {
        sans: Some((Arc::new(FONT.to_vec()), 0)),
        ..DefaultFonts::default()
    });
    set_text_tuning(TextTuning {
        raster: false,
        ..TextTuning::default()
    });
    let viewport = (VIEWPORT.0 as f32, VIEWPORT.1 as f32);
    let mut ui = Harness::new(lines).pixels(VIEWPORT.0, VIEWPORT.1);
    ui.settle();
    let mut checked = 0;
    let mut barred = Vec::new();
    for tenths in SCALES {
        let scene = ui.host().0.emit(viewport);
        let size = [VIEWPORT.0 * tenths / 10, VIEWPORT.1 * tenths / 10];
        let ink = Ink::new(&render_scene_to_image(device, queue, &scene, viewport, size).unwrap());
        for bounds in ink.glyph_boxes() {
            if bounds[3] - bounds[1] < MIN_RING_PX {
                continue;
            }
            checked += 1;
            if ink.counter_inked(bounds) {
                barred.push((tenths, bounds));
            }
        }
    }
    assert!(checked > 0, "no glyph rings found");
    assert!(
        barred.is_empty(),
        "ink across the counter of o (scale x10, bounds): {barred:?}"
    );
}
