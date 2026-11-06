use resvg::usvg::{ClipPath, FillRule as SvgRule, Group, Node, Path, Transform};

use crate::fnv::OFFSET;
use crate::geometry::Rect;
use crate::scene::SceneBuilder;
use crate::scene::command::{
    Clip, CommandScene, DrawPath, FillRule, LayerEffect, Mask, MaskMode, PathPaint, outline_key,
};

use super::group::{Scope, bounds, layer};
use super::paint::OPAQUE;
use super::shape::{outline, scene_transform, transform_key};

const EDGE: f32 = 1e-4;

/// Clips what `scope` later wraps to `clip` under `t`, nesting a clip path's own clip path.
pub(super) fn apply(
    scene: &mut SceneBuilder,
    scope: &mut Scope,
    clip: &ClipPath,
    t: Transform,
    area: Rect,
) {
    let inner = t.pre_concat(clip.transform());
    if let Some(path) = single(clip.root()) {
        scope.clip(scene, Clip::Path(shape(path, inner)));
    } else {
        let mask = Mask::new(MaskMode::Alpha, shapes(clip.root(), inner));
        scope.layer(scene, layer(area, LayerEffect::Mask(mask)));
    }
    if let Some(nested) = clip.clip_path() {
        apply(scene, scope, nested, t, area);
    }
}

/// Whether `group`'s clip path is a plain rectangle that already holds the group's content, so it cuts nothing.
pub(super) fn covers(group: &Group, clip: &ClipPath) -> bool {
    let own = clip.transform();
    let flat = own.kx == 0.0 && own.ky == 0.0 && clip.clip_path().is_none();
    let Some(path) = single(clip.root()).filter(|_| flat) else {
        return false;
    };
    let data = path.data();
    let rect = data.bounds();
    let corners = data.points().iter().all(|point| {
        let near = |a: f32, b: f32| (a - b).abs() <= EDGE;
        (near(point.x, rect.left()) || near(point.x, rect.right()))
            && (near(point.y, rect.top()) || near(point.y, rect.bottom()))
    });
    let (Some(area), content) = (rect.transform(own), group.layer_bounding_box()) else {
        return false;
    };
    corners
        && area.left() <= content.left()
        && area.top() <= content.top()
        && area.right() >= content.right()
        && area.bottom() >= content.bottom()
}

fn single(root: &Group) -> Option<&Path> {
    match root.children() {
        [Node::Path(path)] if path.is_visible() && path.fill().is_some() => Some(path),
        _ => None,
    }
}

fn shapes(root: &Group, t: Transform) -> CommandScene {
    let mut scene = SceneBuilder::new();
    draw(&mut scene, root, t);
    scene.finish()
}

fn draw(scene: &mut SceneBuilder, group: &Group, t: Transform) {
    for child in group.children() {
        match child {
            Node::Path(path) if path.is_visible() && path.fill().is_some() => {
                scene.draw_path(shape(path, t));
            }
            Node::Text(text) => draw(scene, text.flattened(), t),
            Node::Group(child) => nested(scene, child, t.pre_concat(child.transform())),
            Node::Path(_) | Node::Image(_) => {}
        }
    }
}

fn nested(scene: &mut SceneBuilder, group: &Group, t: Transform) {
    let Some(clip) = group.clip_path() else {
        draw(scene, group, t);
        return;
    };
    let Some(area) = bounds(group, t, false) else {
        return;
    };
    let mask = Mask::new(
        MaskMode::Alpha,
        shapes(clip.root(), t.pre_concat(clip.transform())),
    );
    let mut scope = Scope::default();
    scope.layer(scene, layer(area, LayerEffect::Mask(mask)));
    if let Some(further) = clip.clip_path() {
        apply(scene, &mut scope, further, t, area);
    }
    draw(scene, group, t);
    scope.close(scene);
}

fn shape(path: &Path, t: Transform) -> DrawPath {
    let rule = match path.fill().map(resvg::usvg::Fill::rule) {
        Some(SvgRule::EvenOdd) => FillRule::EvenOdd,
        _ => FillRule::NonZero,
    };
    let cmds = outline(path.data());
    let key = transform_key(outline_key(OFFSET, &cmds), t);
    DrawPath::new(cmds, scene_transform(t), key).with_fill(rule, PathPaint::Solid(OPAQUE))
}
