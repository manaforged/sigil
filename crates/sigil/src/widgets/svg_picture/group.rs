use resvg::usvg::{self, Group, Node, Transform};

use crate::geometry::{Point, Rect};
use crate::scene::SceneBuilder;
use crate::scene::command::{
    BlendMode, Clip, LayerEffect, LayerScope, Mask, MaskMode, SceneCommand,
};
use crate::scene::corner_radii::CornerRadii;

use super::filter::filters;
use super::lower::Walk;
use super::{AA_MARGIN, clip};

/// The layers and clips opened around a group's children, closed in reverse.
#[derive(Default)]
pub(super) struct Scope {
    closes: Vec<SceneCommand>,
}

impl Scope {
    pub(super) fn layer(&mut self, scene: &mut SceneBuilder, scope: LayerScope) {
        scene.extend_commands([SceneCommand::PushLayer(scope)]);
        self.closes.push(SceneCommand::PopLayer);
    }

    pub(super) fn clip(&mut self, scene: &mut SceneBuilder, clip: Clip) {
        scene.extend_commands([SceneCommand::PushClip(clip)]);
        self.closes.push(SceneCommand::PopClip);
    }

    pub(super) fn close(self, scene: &mut SceneBuilder) {
        scene.extend_commands(self.closes.into_iter().rev());
    }
}

pub(super) const fn layer(area: Rect, effect: LayerEffect) -> LayerScope {
    LayerScope {
        clip: area,
        clip_radii: CornerRadii::ZERO,
        effect,
        opacity: 1.0,
        blend: BlendMode::SrgbOver,
    }
}

/// The picture-space box of a group's layer under `t`, which already holds the group's transform.
pub(super) fn bounds(group: &Group, t: Transform, filtered: bool) -> Option<Rect> {
    let area = group.layer_bounding_box().transform(t)?;
    let rect = Rect::new(Point::new(area.x(), area.y()), area.width(), area.height());
    Some(if filtered {
        rect
    } else {
        rect.expand(AA_MARGIN)
    })
}

impl Walk {
    pub(super) fn group(&mut self, scene: &mut SceneBuilder, group: &Group, t: Transform) {
        let inner = t.pre_concat(group.transform());
        if group.opacity().get() <= 0.0 {
            return;
        }
        if !group.should_isolate() {
            self.children(scene, group, inner);
            return;
        }
        if let Some(path) = lone_path(group) {
            if let Err(reason) = self.path(scene, path, inner, group.opacity().get()) {
                let node = Node::Group(Box::new(group.clone()));
                self.fallback(scene, &node, t, reason);
            }
            return;
        }
        let effects = match filters(group, inner) {
            Ok(effects) => effects,
            Err(reason) => {
                let node = Node::Group(Box::new(group.clone()));
                self.fallback(scene, &node, t, reason);
                return;
            }
        };
        let Some(area) = bounds(group, inner, !effects.is_empty()) else {
            return;
        };
        let mut scope = Scope::default();
        self.masks(scene, &mut scope, group, (inner, area));
        if let Some(path) = group.clip_path().filter(|path| !clip::covers(group, path)) {
            clip::apply(scene, &mut scope, path, inner, area);
        }
        if !effects.is_empty() {
            scope.layer(scene, layer(area, LayerEffect::Filter(effects.into())));
        }
        self.children(scene, group, inner);
        scope.close(scene);
    }

    fn masks(
        &mut self,
        scene: &mut SceneBuilder,
        scope: &mut Scope,
        group: &Group,
        (t, area): (Transform, Rect),
    ) {
        let mut masks = Vec::new();
        let mut next = group.mask();
        while let Some(mask) = next {
            masks.push(self.mask(mask, t));
            next = mask.mask();
        }
        let mut masks = masks.into_iter();
        let first = masks
            .next()
            .map_or(LayerEffect::Identity, LayerEffect::Mask);
        let blend = blend(group.blend_mode());
        let opacity = group.opacity().get();
        let plain = matches!(first, LayerEffect::Identity)
            && opacity >= 1.0
            && blend == BlendMode::SrgbOver;
        if !plain || group.isolate() {
            scope.layer(
                scene,
                LayerScope {
                    opacity,
                    blend,
                    ..layer(area, first)
                },
            );
        }
        for mask in masks {
            scope.layer(scene, layer(area, LayerEffect::Mask(mask)));
        }
    }

    fn mask(&mut self, mask: &usvg::Mask, t: Transform) -> Mask {
        let mode = match mask.kind() {
            usvg::MaskType::Luminance => MaskMode::Luminance,
            usvg::MaskType::Alpha => MaskMode::Alpha,
        };
        let region = Clip::polygon(&corners(mask.rect(), t));
        let commands = self.detached(|walk, scene| {
            scene.with_clip(region, |scene| walk.children(scene, mask.root(), t));
        });
        Mask::new(mode, commands)
    }
}

/// The one path of a group that only fades it, when that path paints a fill or a stroke but not both, so the fade can go into its paint.
fn lone_path(group: &Group) -> Option<&usvg::Path> {
    let faded_only = group.mask().is_none()
        && group.clip_path().is_none()
        && group.filters().is_empty()
        && group.blend_mode() == usvg::BlendMode::Normal
        && !group.isolate();
    match group.children() {
        [Node::Path(path)]
            if faded_only
                && path.is_visible()
                && path.fill().is_some() != path.stroke().is_some() =>
        {
            Some(path)
        }
        _ => None,
    }
}

fn corners(rect: usvg::NonZeroRect, t: Transform) -> [Point; 4] {
    let mut points = [
        usvg::tiny_skia_path::Point::from_xy(rect.left(), rect.top()),
        usvg::tiny_skia_path::Point::from_xy(rect.right(), rect.top()),
        usvg::tiny_skia_path::Point::from_xy(rect.right(), rect.bottom()),
        usvg::tiny_skia_path::Point::from_xy(rect.left(), rect.bottom()),
    ];
    t.map_points(&mut points);
    points.map(|point| Point::new(point.x, point.y))
}

const fn blend(mode: usvg::BlendMode) -> BlendMode {
    match mode {
        usvg::BlendMode::Normal => BlendMode::SrgbOver,
        usvg::BlendMode::Multiply => BlendMode::Multiply,
        usvg::BlendMode::Screen => BlendMode::Screen,
        usvg::BlendMode::Overlay => BlendMode::Overlay,
        usvg::BlendMode::Darken => BlendMode::Darken,
        usvg::BlendMode::Lighten => BlendMode::Lighten,
        usvg::BlendMode::ColorDodge => BlendMode::ColorDodge,
        usvg::BlendMode::ColorBurn => BlendMode::ColorBurn,
        _ => light_blend(mode),
    }
}

const fn light_blend(mode: usvg::BlendMode) -> BlendMode {
    match mode {
        usvg::BlendMode::HardLight => BlendMode::HardLight,
        usvg::BlendMode::SoftLight => BlendMode::SoftLight,
        usvg::BlendMode::Difference => BlendMode::Difference,
        usvg::BlendMode::Exclusion => BlendMode::Exclusion,
        usvg::BlendMode::Hue => BlendMode::Hue,
        usvg::BlendMode::Saturation => BlendMode::Saturation,
        usvg::BlendMode::Color => BlendMode::Color,
        _ => BlendMode::Luminosity,
    }
}
