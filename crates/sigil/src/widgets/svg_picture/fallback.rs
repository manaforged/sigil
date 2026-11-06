use std::fmt;
use std::sync::Arc;

use resvg::tiny_skia::PixmapMut;
use resvg::usvg::{Node, NonZeroRect, Transform, Tree};

use crate::geometry::{Point, Rect};
use crate::kernel::{ImageRaster, ObjectFit};
use crate::scene::SceneBuilder;
use crate::scene::command::{DrawImage, LayerScope, SceneCommand};

use super::lower::Walk;
use super::{raster, raster_with};

/// Why a subtree of an SVG is drawn as a raster image instead of scene commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Fallback {
    /// A filter primitive with no scene filter, named as in SVG.
    Filter(&'static str),
    /// A filter primitive whose subregion is smaller than its filter region.
    FilterRegion,
    /// A filter under a rotation, skew or uneven scale.
    FilterTransform,
    /// A radial gradient whose focal point is off its center.
    FocalGradient,
    /// A pattern that needs more tiles than the cap.
    PatternTiles,
    /// Lowering work past the per-picture command and node budget.
    Budget,
}

impl fmt::Display for Fallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filter(name) => write!(f, "filter {name}"),
            Self::FilterRegion => f.write_str("filter subregion"),
            Self::FilterTransform => f.write_str("filter under rotation or skew"),
            Self::FocalGradient => f.write_str("focal radial gradient"),
            Self::PatternTiles => f.write_str("pattern tile count"),
            Self::Budget => f.write_str("lowering budget"),
        }
    }
}

/// One `DrawImage` of all of `tree` in a box of `size` at the origin, in a layer that composites in sRGB like resvg.
pub(super) fn bitmap(tree: Arc<Tree>, size: (f32, f32), key: u64) -> Vec<SceneCommand> {
    let Some(source) = raster(key, &tree, size) else {
        return Vec::new();
    };
    #[allow(clippy::cast_precision_loss, reason = "device sides are below 2^24")]
    let image = ImageRaster::new(key, move |width, height, _| {
        raster(key, &tree, (width as f32, height as f32))
    });
    let destination = Rect::new(Point::new(0.0, 0.0), size.0, size.1);
    vec![
        SceneCommand::PushLayer(LayerScope::srgb(destination)),
        SceneCommand::Image(DrawImage {
            node: None,
            source,
            fit: ObjectFit::Fill,
            destination,
            opacity: 10_000,
            crop: None,
            raster: Some(image),
        }),
        SceneCommand::PopLayer,
    ]
}

struct Source {
    node: Arc<Node>,
    shift: Transform,
    dest: Rect,
}

impl Walk {
    pub(super) fn fallback(
        &mut self,
        scene: &mut SceneBuilder,
        node: &Node,
        t: Transform,
        reason: Fallback,
    ) {
        self.fallbacks.push(reason);
        self.raster(scene, node, t);
    }

    pub(super) fn raster(&mut self, scene: &mut SceneBuilder, node: &Node, t: Transform) {
        let (Some(local), Some(abs)) = (local_bounds(node), node.abs_layer_bounding_box()) else {
            return;
        };
        let Some(dest) = local.transform(t) else {
            return;
        };
        let dest = Rect::new(Point::new(dest.x(), dest.y()), dest.width(), dest.height());
        let source = Arc::new(Source {
            node: Arc::new(node.clone()),
            shift: t.pre_translate(abs.x(), abs.y()),
            dest,
        });
        let key = self.next_key();
        let first = Arc::clone(&source);
        let Some(image) = raster_with(key, (dest.width, dest.height), |pixels, size| {
            first.paint(pixels, size);
        }) else {
            return;
        };
        let raster = ImageRaster::new(key, move |width, height, _| {
            #[allow(clippy::cast_precision_loss, reason = "device sides are below 2^24")]
            let size = (width as f32, height as f32);
            raster_with(key, size, |pixels, size| {
                source.paint(pixels, size);
            })
        });
        scene.draw_image(DrawImage {
            node: None,
            source: image,
            fit: ObjectFit::Fill,
            destination: dest,
            opacity: 10_000,
            crop: None,
            raster: Some(raster),
        });
    }
}

impl Source {
    fn paint(&self, pixels: &mut PixmapMut<'_>, (width, height): (f32, f32)) {
        let fit = Transform::from_scale(width / self.dest.width, height / self.dest.height)
            .pre_translate(-self.dest.origin.x, -self.dest.origin.y)
            .pre_concat(self.shift);
        let _drawn = resvg::render_node(&self.node, fit, pixels);
    }
}

fn local_bounds(node: &Node) -> Option<NonZeroRect> {
    match node {
        Node::Group(group) => group.layer_bounding_box().transform(group.transform()),
        Node::Path(path) => path.stroke_bounding_box().to_non_zero_rect(),
        Node::Image(image) => image.bounding_box().to_non_zero_rect(),
        Node::Text(text) => {
            let flat = text.flattened();
            flat.layer_bounding_box().transform(flat.transform())
        }
    }
}
