use std::rc::Rc;

use sigil_hal::{ParagraphMetrics, Point, Rect};
use sigil_kernel::{Cx, Node, NodeId, NodeType, Surface, TextContent};
use sigil_layout::LayoutResult;
use sigil_style::{
    DisplayKind, Elevation, RenderStyle, Rgba8, StyleAtom, StyleBinding, TextDecoration, TextStyle,
    Theme,
};

use crate::corner_radii::CornerRadii;
use crate::scene_builder::SceneBuilder;
use crate::scene_command::{
    CommandScene, DecorationKind, LayerEffect, LayerScope, SceneDecorationStyle, SceneRectStyle,
    Stroke, StrokeAlign, StrokeStyle, shape_glyph_run_with,
};
use crate::shadow_paint::Shadow;
use crate::style_adapter::{corner_radii_for, focus_ring_for, render_style_for, text_style_for};

const VIEW_OVERSCAN: f32 = 64.0;

#[must_use]
pub fn emit_scene(cx: &Cx, root: NodeId) -> CommandScene {
    emit_scene_clipped(cx, root, None)
}

#[must_use]
pub fn emit_scene_clipped(cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene {
    let clip = clip.map(|rect| rect.expand(VIEW_OVERSCAN));
    let mut scene = SceneBuilder::new();
    let inherited = Inherited {
        theme: default_inherited_theme(),
        text_color: None,
        text_style: Rc::new(TextStyle::default()),
    };
    let mut deferred: Vec<(NodeId, Inherited<'_>, Option<Rect>)> = Vec::new();
    let mut walk = EmitWalk {
        scene: &mut scene,
        cx,
        deferred: &mut deferred,
    };
    push_subtree(&mut walk, root, &inherited, clip);
    let mut next = 0;
    while let Some((id, inh, child_clip)) = walk.deferred.get(next).cloned() {
        next += 1;
        push_subtree(&mut walk, id, &inh, child_clip);
    }
    scene.finish()
}

struct EmitWalk<'a, 't> {
    scene: &'a mut SceneBuilder,
    cx: &'t Cx,
    deferred: &'a mut Vec<(NodeId, Inherited<'t>, Option<Rect>)>,
}

#[derive(Clone)]
struct Inherited<'t> {
    theme: &'t Theme,
    text_color: Option<Rgba8>,
    text_style: Rc<TextStyle>,
}

fn push_subtree<'t>(
    walk: &mut EmitWalk<'_, 't>,
    id: NodeId,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
) {
    let Some(node) = walk.cx.get(id) else {
        return;
    };
    if is_display_none(node) {
        return;
    }
    if node.get_extension::<Surface>().is_some() {
        return;
    }
    if outside_clip(node, clip) {
        return;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
    let style = render_style_for(id, node, theme, inherited.text_color);
    let layered_text = node
        .get_extension::<StyleBinding>()
        .and_then(|binding| TextStyle::layered_over_binding(&inherited.text_style, binding));
    let child_inherited = Inherited {
        theme,
        text_color: style.text_color.or(inherited.text_color),
        text_style: layered_text.map_or_else(|| Rc::clone(&inherited.text_style), Rc::new),
    };
    if let Some(result) = node.get_extension::<LayoutResult>() {
        let rect = result.rect();
        let paint = NodePaint {
            style: &style,
            theme,
            text_base: &inherited.text_style,
        };
        let inherit = crate::emit_cache::inherit_key(
            theme,
            inherited.text_color,
            inherited.text_style.as_ref(),
        ) ^ crate::emit_cache::content_key(node);
        let key = (walk.cx.instance_id(), id);
        if !crate::emit_cache::replay(walk.scene, key, node, rect, inherit) {
            let start = walk.scene.command_len();
            push_focus_ring(walk.scene, id, node, rect, &paint);
            push_node(walk.scene, id, node, rect, &paint);
            crate::emit_cache::store(key, node, rect, inherit, walk.scene.copy_from(start));
        }
        if result.clips() {
            emit_clipped_children(walk, node, &child_inherited, clip, rect, &style);
            return;
        }
    }
    push_children(walk, node, &child_inherited, clip);
}

#[allow(
    clippy::too_many_arguments,
    reason = "clip walk needs the node, inherit, and both clip rects"
)]
fn emit_clipped_children<'t>(
    walk: &mut EmitWalk<'_, 't>,
    node: &Node,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
    rect: Rect,
    style: &RenderStyle,
) {
    let Some(inner) = tighten_clip(clip, rect) else {
        return;
    };
    let inner = inner.expand(1.0);
    let clip_radii = corner_radii_for(style.border_radius);
    let cx = walk.cx;
    let deferred = &mut *walk.deferred;
    if clip_radii.0.iter().any(|radius| *radius > 0.0) {
        walk.scene.with_composite_layer(
            LayerScope {
                clip: rect,
                clip_radii,
                effect: LayerEffect::Identity,
                opacity: 1.0,
            },
            |scene| {
                let mut nested = EmitWalk {
                    scene,
                    cx,
                    deferred,
                };
                push_children(&mut nested, node, inherited, Some(inner));
            },
        );
    } else {
        walk.scene.with_clip(rect, |scene| {
            let mut nested = EmitWalk {
                scene,
                cx,
                deferred,
            };
            push_children(&mut nested, node, inherited, Some(inner));
        });
    }
}

fn outside_clip(node: &Node, clip: Option<Rect>) -> bool {
    let Some(clip) = clip else {
        return false;
    };
    let Some(result) = node.get_extension::<LayoutResult>() else {
        return false;
    };
    !result.rect().intersects(clip)
}

fn tighten_clip(clip: Option<Rect>, rect: Rect) -> Option<Rect> {
    clip.map_or(Some(rect), |clip| clip.intersection(rect))
}

fn push_children<'t>(
    walk: &mut EmitWalk<'_, 't>,
    node: &Node,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
) {
    for child in node.children().iter().copied() {
        let is_top_layer = walk
            .cx
            .get(child)
            .is_some_and(|n| n.get_extension::<sigil_kernel::TopLayer>().is_some());
        if is_top_layer {
            walk.deferred.push((child, inherited.clone(), clip));
        } else {
            push_subtree(walk, child, inherited, clip);
        }
    }
}

fn default_inherited_theme<'theme>() -> &'theme Theme {
    use std::sync::OnceLock;
    static DEFAULT: OnceLock<Theme> = OnceLock::new();
    DEFAULT.get_or_init(Theme::default)
}

fn is_display_none(node: &Node) -> bool {
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return false;
    };
    let target = StyleAtom::Display(DisplayKind::None);
    binding.static_slice().contains(&target)
        || binding.dynamic_slice().contains(&target)
        || binding.reactive_contains(&target)
}

fn push_focus_ring(
    scene: &mut SceneBuilder,
    id: NodeId,
    node: &Node,
    rect: Rect,
    paint: &NodePaint<'_>,
) {
    let Some(ring) = focus_ring_for(node, paint.theme).filter(|ring| ring.width > 0.0) else {
        return;
    };
    let offset = ring.offset;
    let ring_rect = Rect::new(
        Point::new(rect.origin.x - offset, rect.origin.y - offset),
        offset.mul_add(2.0, rect.width),
        offset.mul_add(2.0, rect.height),
    );
    let radii = CornerRadii(
        corner_radii_for(paint.style.border_radius)
            .0
            .map(|radius| radius + offset),
    );
    let transparent = Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    scene.draw_rect_for_node(
        id,
        ring_rect,
        SceneRectStyle::new(transparent, radii, ring.opacity).with_stroke(Stroke {
            color: ring.color,
            width: ring.width,
            style: StrokeStyle::Solid,
            align: StrokeAlign::Outside,
        }),
    );
}

struct NodePaint<'a> {
    style: &'a RenderStyle,
    theme: &'a Theme,
    text_base: &'a TextStyle,
}

fn push_text_decoration(
    scene: &mut SceneBuilder,
    rect: Rect,
    color: Rgba8,
    opacity: u16,
    text_style: &TextStyle,
) {
    let kind = match text_style.decoration {
        TextDecoration::None => return,
        TextDecoration::Underline => DecorationKind::Underline,
        TextDecoration::LineThrough => DecorationKind::Strikethrough,
    };
    let thickness = (text_style.font_size * 0.06).max(1.0);
    let y = match kind {
        DecorationKind::Strikethrough => rect.origin.y + (rect.height - thickness) / 2.0,
        DecorationKind::Underline | DecorationKind::WavyUnderline => {
            rect.origin.y + rect.height - thickness
        }
    };
    scene.draw_decoration(SceneDecorationStyle {
        kind,
        band: Rect::new(Point::new(rect.origin.x, y), rect.width, thickness),
        color,
        opacity,
        period: 0.0,
        line_width: thickness,
    });
}

fn push_container(scene: &mut SceneBuilder, id: NodeId, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    if let Some(elevation) = style.shadow {
        scene.draw_shadow(
            rect,
            corner_radii_for(style.border_radius),
            elevation_shadow(elevation),
        );
    }
    let stroke = border_stroke(style, paint.theme);
    if style.background_color.is_some() || stroke.is_some() {
        let fill = style.background_color.unwrap_or(Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
        let mut rect_style =
            SceneRectStyle::new(fill, corner_radii_for(style.border_radius), style.opacity);
        if let Some(stroke) = stroke {
            rect_style = rect_style.with_stroke(stroke);
        }
        scene.draw_rect_for_node(id, rect, rect_style);
    }
    push_side_borders(scene, id, rect, style, paint.theme);
}

fn push_node(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    match &node.kind {
        NodeType::Container => push_container(scene, id, rect, paint),
        NodeType::Text { .. } => push_text(scene, id, node, rect, paint),
        NodeType::Image { source, fit } => {
            scene.draw_image(crate::scene_command::DrawImage {
                node: Some(id),
                source: source.clone(),
                fit: *fit,
                destination: rect,
                opacity: style.opacity,
            });
        }
        NodeType::Fragment => {}
    }
}

fn push_text(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    let NodeType::Text {
        content, ellipsis, ..
    } = &node.kind
    else {
        return;
    };
    let Some(color) = style.text_color else {
        return;
    };
    let text_style = text_style_for(node, paint.theme, paint.text_base);
    let run = shape_glyph_run_with(
        rect,
        text_content(content),
        color,
        text_style.clone(),
        style.opacity,
        *ellipsis,
    );
    let overflows = *ellipsis
        && run.paragraph().is_some_and(|paragraph| {
            paragraph.width() > rect.width + 0.5 || paragraph.height() > rect.height + 0.5
        });
    let emit = |target: &mut SceneBuilder| {
        if let Some(paragraph) = run.paragraph() {
            crate::color_glyph_paint::emit_color_glyphs(target, id, rect, paragraph, style.opacity);
        }
        target.draw_glyph_run_for_node(id, run);
    };
    if overflows {
        scene.with_clip(rect, emit);
    } else {
        emit(scene);
    }
    push_text_decoration(scene, rect, color, style.opacity, &text_style);
}

const fn elevation_shadow(elevation: Elevation) -> Shadow {
    let (offset_y, blur, spread, alpha) = match elevation {
        Elevation::Sm => (1.0, 2.0, 0.0, 0.10),
        Elevation::Base => (2.0, 4.0, 0.0, 0.12),
        Elevation::Md => (4.0, 8.0, -1.0, 0.14),
        Elevation::Lg => (8.0, 16.0, -2.0, 0.16),
        Elevation::Xl => (16.0, 28.0, -4.0, 0.20),
    };
    Shadow::new([0.0, offset_y], blur, spread, [0.0, 0.0, 0.0, alpha])
}

fn border_edges(style: &RenderStyle) -> [f32; 4] {
    let uniform = style.border_width.map_or(0.0, |px| f32::from(px.0));
    [
        style.border_top.map_or(uniform, |px| f32::from(px.0)),
        style.border_right.map_or(uniform, |px| f32::from(px.0)),
        style.border_bottom.map_or(uniform, |px| f32::from(px.0)),
        style.border_left.map_or(uniform, |px| f32::from(px.0)),
    ]
}

fn border_color(style: &RenderStyle, theme: &Theme) -> Rgba8 {
    style
        .border_color
        .or(style.text_color)
        .unwrap_or(theme.on_surface)
}

const fn borders_uniform([top, right, bottom, left]: [f32; 4]) -> bool {
    top.to_bits() == right.to_bits()
        && right.to_bits() == bottom.to_bits()
        && bottom.to_bits() == left.to_bits()
}

fn border_stroke(style: &RenderStyle, theme: &Theme) -> Option<Stroke> {
    let [top, right, bottom, left] = border_edges(style);
    if top <= 0.0 || !borders_uniform([top, right, bottom, left]) {
        return None;
    }
    Some(Stroke {
        color: border_color(style, theme),
        width: top,
        style: StrokeStyle::Solid,
        align: StrokeAlign::Inside,
    })
}

fn push_side_borders(
    scene: &mut SceneBuilder,
    id: NodeId,
    rect: Rect,
    style: &RenderStyle,
    theme: &Theme,
) {
    let [top, right, bottom, left] = border_edges(style);
    if top <= 0.0 && right <= 0.0 && bottom <= 0.0 && left <= 0.0 {
        return;
    }
    if top > 0.0 && borders_uniform([top, right, bottom, left]) {
        return;
    }
    let color = border_color(style, theme);
    let fill = SceneRectStyle::new(color, CornerRadii::new(0.0, 0.0, 0.0, 0.0), style.opacity);
    if top > 0.0 {
        scene.draw_rect_for_node(
            id,
            Rect::new(rect.origin, rect.width, top.min(rect.height)),
            fill,
        );
    }
    if bottom > 0.0 {
        let height = bottom.min(rect.height);
        scene.draw_rect_for_node(
            id,
            Rect::new(
                Point::new(rect.origin.x, rect.origin.y + rect.height - height),
                rect.width,
                height,
            ),
            fill,
        );
    }
    if left > 0.0 {
        scene.draw_rect_for_node(
            id,
            Rect::new(rect.origin, left.min(rect.width), rect.height),
            fill,
        );
    }
    if right > 0.0 {
        let width = right.min(rect.width);
        scene.draw_rect_for_node(
            id,
            Rect::new(
                Point::new(rect.origin.x + rect.width - width, rect.origin.y),
                width,
                rect.height,
            ),
            fill,
        );
    }
}

fn text_content(content: &TextContent) -> String {
    content.resolve()
}
