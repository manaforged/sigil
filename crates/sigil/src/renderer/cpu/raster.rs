#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::hal::{Point, Rect};
use crate::icons::{IconBaker, builtin};
use crate::style::Rgba8;
use ahash::AHashMap;
use tiny_skia::{FilterQuality, Paint, Pattern, Pixmap, SpreadMode, Transform};

use crate::renderer::scene_command::{DrawImage, DrawSprite, PatternKind, SpriteKind};
use crate::renderer::scenes::BUILTIN_ICON_ATLAS_BASE_ID;
use crate::renderer::sprite_atlas::{
    BUILTIN_ATLAS_HEIGHT, BUILTIN_ATLAS_WIDTH, builtin_atlas_rgba8,
};
use crate::renderer::sprite_paint::BUILTIN_SPRITE_ATLAS_ID;

use super::shapes::{Brush, solid};

const ICON_PX: u32 = 128;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct SpriteKey {
    atlas: u32,
    region: [u32; 4],
    color: Rgba8,
    kind: u8,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct TileKey {
    kind: u8,
    a: Rgba8,
    b: Rgba8,
    size: u32,
    line: u32,
    scale: u32,
}

#[derive(Default)]
pub(super) struct RasterCache {
    images: AHashMap<u64, Pixmap>,
    atlases: AHashMap<u32, Pixmap>,
    sprites: AHashMap<SpriteKey, Pixmap>,
    tiles: AHashMap<TileKey, Pixmap>,
}

pub(super) fn from_rgba8(width: u32, height: u32, rgba: &[u8]) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width, height)?;
    let (dst_px, _) = pixmap.data_mut().as_chunks_mut::<4>();
    let (src_px, _) = rgba.as_chunks::<4>();
    for (dst, src) in dst_px.iter_mut().zip(src_px) {
        let alpha = u32::from(src[3]);
        for (d, s) in dst.iter_mut().zip(src).take(3) {
            *d = ((u32::from(*s) * alpha + 127) / 255) as u8;
        }
        dst[3] = src[3];
    }
    Some(pixmap)
}

fn fill_scaled(target: &mut Pixmap, source: &Pixmap, src: Rect, dest: Rect, brush: &Brush<'_>) {
    let Some(dest_rect) =
        tiny_skia::Rect::from_xywh(dest.origin.x, dest.origin.y, dest.width, dest.height)
    else {
        return;
    };
    let scale_x = dest.width / src.width.max(f32::EPSILON);
    let scale_y = dest.height / src.height.max(f32::EPSILON);
    let map = Transform::from_row(
        scale_x,
        0.0,
        0.0,
        scale_y,
        src.origin.x.mul_add(-scale_x, dest.origin.x),
        src.origin.y.mul_add(-scale_y, dest.origin.y),
    );
    let paint = Paint {
        shader: Pattern::new(
            source.as_ref(),
            SpreadMode::Pad,
            FilterQuality::Bilinear,
            brush.opacity,
            map,
        ),
        anti_alias: true,
        ..Paint::default()
    };
    target.fill_rect(dest_rect, &paint, brush.transform, brush.mask);
}

impl RasterCache {
    pub(super) fn image(
        &mut self,
        target: &mut Pixmap,
        draw: &DrawImage,
        dest: Rect,
        brush: &Brush<'_>,
    ) {
        let source = &draw.source;
        if !self.images.contains_key(&source.id()) {
            let Some(pixmap) = from_rgba8(source.width(), source.height(), source.rgba()) else {
                return;
            };
            self.images.insert(source.id(), pixmap);
        }
        let Some(pixmap) = self.images.get(&source.id()) else {
            return;
        };
        let (src_rect, dst_rect) = fit(
            draw.fit,
            pixmap.width() as f32,
            pixmap.height() as f32,
            dest,
        );
        fill_scaled(target, pixmap, src_rect, dst_rect, brush);
    }

    pub(super) fn sprite(
        &mut self,
        target: &mut Pixmap,
        draw: &DrawSprite,
        dest: Rect,
        brush: &Brush<'_>,
    ) {
        let region = draw.source.region;
        let key = SpriteKey {
            atlas: draw.source.atlas,
            region: [
                region.origin.x,
                region.origin.y,
                region.width,
                region.height,
            ]
            .map(f32::to_bits),
            color: draw.color,
            kind: kind_index(draw.kind),
        };
        if !self.sprites.contains_key(&key) {
            let Some(atlas) = self.atlas(draw.source.atlas) else {
                return;
            };
            let Some(tinted) = tint(atlas, region, draw.color, draw.kind) else {
                return;
            };
            self.sprites.insert(key, tinted);
        }
        let Some(sprite) = self.sprites.get(&key) else {
            return;
        };
        let src = Rect::new(
            Point::new(0.0, 0.0),
            sprite.width() as f32,
            sprite.height() as f32,
        );
        fill_scaled(target, sprite, src, dest, brush);
    }

    fn atlas(&mut self, id: u32) -> Option<&Pixmap> {
        if !self.atlases.contains_key(&id) {
            let pixmap = if id == BUILTIN_SPRITE_ATLAS_ID {
                from_rgba8(
                    BUILTIN_ATLAS_WIDTH,
                    BUILTIN_ATLAS_HEIGHT,
                    &builtin_atlas_rgba8(),
                )?
            } else {
                let index = usize::try_from(id.checked_sub(BUILTIN_ICON_ATLAS_BASE_ID)?).ok()?;
                let (_, svg) = builtin::ALL.get(index)?;
                let icon = IconBaker::new().bake(svg, ICON_PX, ICON_PX).ok()?;
                let rgba: Vec<u8> = icon
                    .alpha
                    .iter()
                    .flat_map(|&a| [255, 255, 255, a])
                    .collect();
                from_rgba8(icon.width, icon.height, &rgba)?
            };
            self.atlases.insert(id, pixmap);
        }
        self.atlases.get(&id)
    }

    pub(super) fn tile(&mut self, pattern: PatternKind, scale: f32) -> Option<&Pixmap> {
        let key = tile_key(pattern, scale);
        if !self.tiles.contains_key(&key) {
            self.tiles.insert(key, build_tile(pattern, scale)?);
        }
        self.tiles.get(&key)
    }
}

const fn kind_index(kind: SpriteKind) -> u8 {
    match kind {
        SpriteKind::Mono => 0,
        SpriteKind::Subpixel => 1,
        SpriteKind::Poly => 2,
    }
}

fn tint(atlas: &Pixmap, region: Rect, color: Rgba8, kind: SpriteKind) -> Option<Pixmap> {
    let left = region.origin.x.max(0.0) as u32;
    let top = region.origin.y.max(0.0) as u32;
    let width = (region.width.round() as u32)
        .min(atlas.width().saturating_sub(left))
        .max(1);
    let height = (region.height.round() as u32)
        .min(atlas.height().saturating_sub(top))
        .max(1);
    let mut out = Pixmap::new(width, height)?;
    let stride = atlas.width() as usize;
    let (src, _) = atlas.data().as_chunks::<4>();
    let tint = [color.r, color.g, color.b].map(u32::from);
    let alpha = u32::from(color.a);
    let (dst, _) = out.data_mut().as_chunks_mut::<4>();
    for row in 0..height as usize {
        for col in 0..width as usize {
            let Some(px) = src.get((top as usize + row) * stride + left as usize + col) else {
                continue;
            };
            let Some(px_out) = dst.get_mut(row * width as usize + col) else {
                continue;
            };
            if matches!(kind, SpriteKind::Poly) {
                *px_out = *px;
                continue;
            }
            let cov = u32::from(px[3]) * alpha / 255;
            for (d, t) in px_out.iter_mut().zip(tint) {
                *d = ((t * cov + 127) / 255) as u8;
            }
            px_out[3] = cov as u8;
        }
    }
    Some(out)
}

const fn tile_key(pattern: PatternKind, scale: f32) -> TileKey {
    match pattern {
        PatternKind::Slash {
            color,
            period,
            line_width,
        } => TileKey {
            kind: 0,
            a: color,
            b: color,
            size: period.to_bits(),
            line: line_width.to_bits(),
            scale: scale.to_bits(),
        },
        PatternKind::Checker {
            color_a,
            color_b,
            cell,
        } => TileKey {
            kind: 1,
            a: color_a,
            b: color_b,
            size: cell.to_bits(),
            line: 0,
            scale: scale.to_bits(),
        },
    }
}

fn build_tile(pattern: PatternKind, scale: f32) -> Option<Pixmap> {
    let identity = Brush {
        transform: Transform::from_scale(scale, scale),
        mask: None,
        opacity: 1.0,
    };
    match pattern {
        PatternKind::Slash {
            color,
            period,
            line_width,
        } => {
            let period = period.max(1.0);
            let px = (period * scale).ceil().max(1.0) as u32;
            let mut tile = Pixmap::new(px, px)?;
            let stroke = tiny_skia::Stroke {
                width: (line_width / std::f32::consts::SQRT_2).max(0.5),
                ..tiny_skia::Stroke::default()
            };
            let paint = solid(color, 1.0);
            for step in -2..=2 {
                let shift = step as f32 * period;
                let mut b = tiny_skia::PathBuilder::new();
                b.move_to(shift - period, -period);
                b.line_to(period.mul_add(2.0, shift), 2.0 * period);
                if let Some(path) = b.finish() {
                    tile.stroke_path(&path, &paint, &stroke, identity.transform, None);
                }
            }
            Some(tile)
        }
        PatternKind::Checker {
            color_a,
            color_b,
            cell,
        } => {
            let cell = cell.max(1.0);
            let px = (cell * scale).ceil().max(1.0) as u32;
            let mut tile = Pixmap::new(px * 2, px * 2)?;
            tile.fill(super::shapes::ts_color(color_b, 1.0));
            let paint = solid(color_a, 1.0);
            for (x, y) in [(0.0, 0.0), (cell, cell)] {
                if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, cell, cell) {
                    tile.fill_rect(rect, &paint, identity.transform, None);
                }
            }
            Some(tile)
        }
    }
}

fn fit(fit: crate::kernel::ObjectFit, iw: f32, ih: f32, dest: Rect) -> (Rect, Rect) {
    let full = Rect::new(Point::new(0.0, 0.0), iw, ih);
    if iw <= 0.0 || ih <= 0.0 || dest.width <= 0.0 || dest.height <= 0.0 {
        return (full, dest);
    }
    match fit {
        crate::kernel::ObjectFit::Fill => (full, dest),
        crate::kernel::ObjectFit::Contain => {
            let s = (dest.width / iw).min(dest.height / ih);
            let (w, h) = (iw * s, ih * s);
            let origin = Point::new(
                dest.origin.x + (dest.width - w) / 2.0,
                dest.origin.y + (dest.height - h) / 2.0,
            );
            (full, Rect::new(origin, w, h))
        }
        crate::kernel::ObjectFit::Cover => {
            let s = (dest.width / iw).max(dest.height / ih);
            let (vw, vh) = (dest.width / s, dest.height / s);
            (
                Rect::new(Point::new((iw - vw) / 2.0, (ih - vh) / 2.0), vw, vh),
                dest,
            )
        }
    }
}
