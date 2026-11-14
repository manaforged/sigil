use crate::scene::command::{DrawPath, PathCommand, Transform};

#[cfg(feature = "scene")]
mod vello {
    use std::cell::RefCell;

    use vello_cpu::{PixmapMut, RasterizerSettings, RenderContext, Resources};

    use super::{DrawPath, Transform};

    thread_local! {
        static CONTEXT: RefCell<Option<(RenderContext, Resources)>> = const { RefCell::new(None) };
    }

    pub(super) fn raster(
        draw: &DrawPath,
        device: Transform,
        (width, height): (u16, u16),
        opacity: f32,
    ) -> Option<Vec<u8>> {
        let mut pixels = vec![0_u8; usize::from(width) * usize::from(height) * 4];
        CONTEXT.with_borrow_mut(|slot| {
            let (context, resources) =
                slot.get_or_insert_with(|| (RenderContext::new(width, height), Resources::new()));
            context.reset_and_resize(width, height);
            crate::renderer::vector::record(context, draw, device, opacity);
            context.flush();
            let target = PixmapMut::new(width, height, &mut pixels)?;
            context.render_with(target, resources, RasterizerSettings::default());
            Some(())
        })?;
        Some(pixels)
    }
}

pub(crate) mod gradient;

/// Builds the `tiny_skia` path of `commands`, or `None` when they enclose nothing.
#[cfg(any(not(feature = "scene"), all(feature = "cpu", host)))]
pub(crate) fn skia_path(commands: &[PathCommand]) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo(x, y) => builder.move_to(x, y),
            PathCommand::LineTo(x, y) => builder.line_to(x, y),
            PathCommand::QuadTo(cx, cy, x, y) => builder.quad_to(cx, cy, x, y),
            PathCommand::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                builder.cubic_to(c1x, c1y, c2x, c2y, x, y);
            }
            PathCommand::Close => builder.close(),
        }
    }
    builder.finish()
}
#[cfg(not(feature = "scene"))]
mod skia;

/// Rasterizes `draw` under `device` into premultiplied RGBA8 of `size`.
pub(crate) fn raster(
    draw: &DrawPath,
    device: Transform,
    size: (u16, u16),
    opacity: f32,
) -> Option<Vec<u8>> {
    if size.0 == 0 || size.1 == 0 {
        return None;
    }
    #[cfg(feature = "scene")]
    let pixels = vello::raster(draw, device, size, opacity);
    #[cfg(not(feature = "scene"))]
    let pixels = skia::raster(draw, device, size, opacity);
    pixels
}
