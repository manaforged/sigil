#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "pixel coordinates are bounded by the target size; float-to-int rounding is the rasterizer contract"
)]

use crate::geometry::{Point, Rect};
use crate::icons::{IconBaker, builtin};
use crate::renderer::cache_limits::CacheLimits;
use crate::style::Rgba8;
use ahash::AHashMap;
use tiny_skia::{FilterQuality, Paint, Pattern, Pixmap, SpreadMode, Transform};

use crate::renderer::scene_command::{DrawImage, DrawSprite, SpriteKind};
use crate::renderer::sprite_atlas::{
    BUILTIN_ATLAS_HEIGHT, BUILTIN_ATLAS_WIDTH, builtin_atlas_rgba8, builtin_icon_index,
};
use crate::renderer::sprite_paint::BUILTIN_SPRITE_ATLAS_ID;

use super::bound::Bound;
use super::pattern::{Placement, shader};
use super::shapes::{Brush, decode, stored};
use super::sprite_tint::tint;

const ICON_PX: u32 = 128;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct SpriteKey {
    atlas: u32,
    region: [u32; 4],
    size: u32,
    color: Rgba8,
    kind: u8,
}

#[derive(Default)]
pub(super) struct RasterCache {
    images: AHashMap<u64, (Pixmap, u64)>,
    frame: u64,
    atlases: AHashMap<u32, Pixmap>,
    sprites: AHashMap<SpriteKey, Pixmap>,
    pattern: Option<Pixmap>,
}

pub(super) fn from_rgba8(width: u32, height: u32, rgba: &[u8]) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width, height)?;
    let (dst_px, _) = pixmap.data_mut().as_chunks_mut::<4>();
    let (src_px, _) = rgba.as_chunks::<4>();
    for (dst, src) in dst_px.iter_mut().zip(src_px) {
        let alpha = f32::from(src[3]) / 255.0;
        for (d, s) in dst.iter_mut().zip(src).take(3) {
            *d = stored(decode(*s), alpha);
        }
        dst[3] = src[3];
    }
    Some(pixmap)
}

fn fill_scaled(target: &mut Pixmap, source: &Pixmap, src: Rect, dest: Rect, brush: &Brush<'_>) {
    let Some(dest_rect) = Bound::of(target, brush.transform).clip_rect(dest) else {
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
        colorspace: super::shapes::BLEND_SPACE,
        ..Paint::default()
    };
    target.fill_rect(dest_rect, &paint, brush.transform, brush.coverage());
}

impl RasterCache {
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        self.images
            .retain(|_, (_, used)| *used + CacheLimits::DEFAULT.image_stale_frames >= frame);
    }

    pub(super) fn image(
        &mut self,
        target: &mut Pixmap,
        draw: &DrawImage,
        scale: f32,
        brush: &Brush<'_>,
    ) {
        let Some((source, src_rect, dst_rect)) = draw.placement_at(scale) else {
            return;
        };
        if !self.images.contains_key(&source.id()) {
            let Some(pixmap) = from_rgba8(source.width(), source.height(), source.rgba()) else {
                return;
            };
            self.images.insert(source.id(), (pixmap, self.frame));
        }
        let Some((pixmap, used)) = self.images.get_mut(&source.id()) else {
            return;
        };
        *used = self.frame;
        let pixmap = &*pixmap;
        fill_scaled(target, pixmap, src_rect, dst_rect, brush);
    }

    pub(super) fn sprite(
        &mut self,
        target: &mut Pixmap,
        draw: &DrawSprite,
        dest: Rect,
        brush: &Brush<'_>,
    ) {
        let Some(dims) = self
            .atlas(draw.source.atlas)
            .map(|atlas| (atlas.width() as f32, atlas.height() as f32))
        else {
            return;
        };
        let Some((region, dest)) = draw.source.fitted(dims, dest) else {
            return;
        };
        let key = SpriteKey {
            atlas: draw.source.atlas,
            region: [
                region.origin.x,
                region.origin.y,
                region.width,
                draw.source.phase,
            ]
            .map(f32::to_bits),
            size: region.height.to_bits(),
            color: draw.color,
            kind: kind_index(draw.kind),
        };
        if !self.sprites.contains_key(&key) {
            let Some(atlas) = self.atlas(draw.source.atlas) else {
                return;
            };
            let Some(tinted) = tint(atlas, region, draw) else {
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
                let index = builtin_icon_index(id)?;
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

    pub(super) fn pattern(&mut self, place: &Placement) -> Option<tiny_skia::Shader<'_>> {
        shader(&mut self.pattern, place)
    }
}

const fn kind_index(kind: SpriteKind) -> u8 {
    match kind {
        SpriteKind::Mono => 0,
        SpriteKind::Subpixel => 1,
        SpriteKind::Poly => 2,
    }
}
