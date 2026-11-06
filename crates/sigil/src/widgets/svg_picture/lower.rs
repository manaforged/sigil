use std::sync::Arc;

use resvg::usvg::tiny_skia_path::Path as Outline;
use resvg::usvg::{self, FillRule as SvgRule, Group, Node, PaintOrder, Transform, Tree};

use crate::fnv::mix;
use crate::geometry::{Point, Rect};
use crate::scene::SceneBuilder;
use crate::scene::command::{
    CommandScene, DrawPath, FillRule, LayerScope, PathCmd, PathPaint, PathStroke, SceneCommand,
    outline_key,
};

use super::budget::{self, MAX_COMMANDS};
use super::fallback::{Fallback, bitmap};
use super::paint::{self, Fill, OPAQUE};
use super::shape::{self, outline, scene_transform, transform_key};

#[cfg(feature = "kittest")]
mod probe;
#[cfg(feature = "kittest")]
pub use probe::{SvgMode, svg_scene};

/// How a picture reaches the scene: lowered to scene commands or as one bitmap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Draw {
    /// Scene commands, with a bitmap only for subtrees that have no scene equivalent.
    Vector,
    /// One resvg bitmap, rastered again at the device size.
    Bitmap,
}

/// The scene commands of one SVG tree and the subtrees that fell back to a raster.
pub(crate) struct Lowered {
    pub(crate) commands: Vec<SceneCommand>,
    pub(crate) fallbacks: Vec<Fallback>,
}

/// The state of one walk over a usvg tree.
pub(super) struct Walk {
    key: u64,
    count: u64,
    pub(super) spent: usize,
    pub(super) exhausted: bool,
    pub(super) fallbacks: Vec<Fallback>,
}

/// Lowers `tree` into a box of `size` at the origin, in one layer that composites over the backdrop in sRGB like resvg.
pub(super) fn lower(tree: &Arc<Tree>, (width, height): (f32, f32), key: u64) -> Lowered {
    if !budget::fits(tree) {
        return whole(tree, (width, height), key);
    }
    let size = tree.size();
    let base = Transform::from_scale(width / size.width(), height / size.height());
    let mut walk = Walk {
        key,
        count: 0,
        spent: 0,
        exhausted: false,
        fallbacks: Vec::new(),
    };
    let mut scene = SceneBuilder::new();
    let area = Rect::new(Point::new(0.0, 0.0), width, height);
    scene.extend_commands([SceneCommand::PushLayer(LayerScope::srgb(area))]);
    walk.group(&mut scene, tree.root(), base);
    scene.pop_layer();
    if scene.command_len() > MAX_COMMANDS {
        return whole(tree, (width, height), key);
    }
    Lowered {
        commands: scene.finish().commands,
        fallbacks: walk.fallbacks,
    }
}

fn whole(tree: &Arc<Tree>, size: (f32, f32), key: u64) -> Lowered {
    Lowered {
        commands: bitmap(Arc::clone(tree), size, key),
        fallbacks: vec![Fallback::Budget],
    }
}

impl Walk {
    pub(super) const fn next_key(&mut self) -> u64 {
        self.count += 1;
        mix(self.key, self.count)
    }

    pub(super) fn detached(
        &mut self,
        body: impl FnOnce(&mut Self, &mut SceneBuilder),
    ) -> CommandScene {
        let mut scene = SceneBuilder::new();
        body(self, &mut scene);
        scene.finish()
    }

    pub(super) fn children(&mut self, scene: &mut SceneBuilder, group: &Group, t: Transform) {
        for node in group.children() {
            self.node(scene, node, t);
        }
    }

    fn node(&mut self, scene: &mut SceneBuilder, node: &Node, t: Transform) {
        match node {
            Node::Group(group) => self.group(scene, group, t),
            Node::Path(path) if path.is_visible() => {
                if let Err(reason) = self.path(scene, path, t) {
                    self.fallback(scene, node, t, reason);
                }
            }
            Node::Path(_) => {}
            Node::Image(_) => self.raster(scene, node, t),
            Node::Text(text) => self.group(scene, text.flattened(), t),
        }
    }

    fn path(
        &mut self,
        scene: &mut SceneBuilder,
        path: &usvg::Path,
        t: Transform,
    ) -> Result<(), Fallback> {
        let fill = path
            .fill()
            .map(|fill| {
                paint::fill(fill.paint(), fill.opacity().get())
                    .map(|paint| (rule(fill.rule()), paint))
            })
            .transpose()?;
        let stroke = path
            .stroke()
            .map(|stroke| {
                paint::fill(stroke.paint(), stroke.opacity().get()).map(|paint| (stroke, paint))
            })
            .transpose()?;
        let data = path.data();
        if path.paint_order() == PaintOrder::StrokeAndFill {
            if let Some((stroke, paint)) = stroke {
                self.stroke(scene, (data, stroke), paint, t)?;
            }
            if let Some((rule, paint)) = fill {
                self.fill(scene, (data, rule), paint, t)?;
            }
            return Ok(());
        }
        if let Some((rule, paint)) = fill {
            self.fill(scene, (data, rule), paint, t)?;
        }
        if let Some((stroke, paint)) = stroke {
            self.stroke(scene, (data, stroke), paint, t)?;
        }
        Ok(())
    }

    fn fill(
        &mut self,
        scene: &mut SceneBuilder,
        (data, rule): (&Outline, FillRule),
        paint: Fill,
        t: Transform,
    ) -> Result<(), Fallback> {
        match paint {
            Fill::Plain(paint) => {
                scene.draw_path(self.draw(outline(data), t).with_fill(rule, paint));
            }
            Fill::Space(paint, space) => {
                let local = space
                    .invert()
                    .and_then(|inverse| data.clone().transform(inverse));
                if let Some(local) = local {
                    let draw = self.draw(outline(&local), t.pre_concat(space));
                    scene.draw_path(draw.with_fill(rule, paint));
                }
            }
            Fill::Pattern(pattern, opacity) => {
                let area = self
                    .draw(outline(data), t)
                    .with_fill(rule, PathPaint::Solid(OPAQUE));
                self.pattern(scene, (&pattern, opacity), area, t)?;
            }
        }
        Ok(())
    }

    fn stroke(
        &mut self,
        scene: &mut SceneBuilder,
        (data, stroke): (&Outline, &usvg::Stroke),
        paint: Fill,
        t: Transform,
    ) -> Result<(), Fallback> {
        if let (Fill::Plain(plain), Some(native)) = (&paint, shape::native(data, stroke)) {
            let line = PathStroke {
                paint: plain.clone(),
                width: stroke.width().get(),
                cap: native.cap,
                join: native.join,
                dash: native.dash,
            };
            scene.draw_path(self.draw(outline(data), t).with_stroke(line));
            return Ok(());
        }
        shape::stroked(data, stroke, t).map_or(Ok(()), |outlined| {
            self.fill(scene, (&outlined, FillRule::NonZero), paint, t)
        })
    }

    fn draw(&mut self, cmds: Arc<[PathCmd]>, t: Transform) -> DrawPath {
        let key = transform_key(outline_key(self.next_key(), &cmds), t);
        DrawPath::new(cmds, scene_transform(t), key)
    }
}

const fn rule(rule: SvgRule) -> FillRule {
    match rule {
        SvgRule::NonZero => FillRule::NonZero,
        SvgRule::EvenOdd => FillRule::EvenOdd,
    }
}
