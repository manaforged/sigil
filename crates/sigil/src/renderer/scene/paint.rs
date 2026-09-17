use crate::hal::{ParagraphMetrics, Point, Rect};
use crate::kernel::{Node, NodeId, NodeType, TextContent};
use crate::renderer::corner_radii::CornerRadii;
use crate::renderer::scene_builder::SceneBuilder;
use crate::renderer::scene_command::{
    DecorationKind, SceneDecorationStyle, SceneRectStyle, Stroke, StrokeAlign, StrokeStyle,
    shape_glyph_run_with,
};
use crate::renderer::shadow_paint::Shadow;
use crate::renderer::style_adapter::{corner_radii_for, focus_ring_for, text_style_for};
use crate::style::{Elevation, RenderStyle, Rgba8, TextDecoration, TextStyle, Theme};

pub(super) fn push_focus_ring(
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

pub(super) struct NodePaint<'a> {
    pub(super) style: &'a RenderStyle,
    pub(super) theme: &'a Theme,
    pub(super) text_base: &'a TextStyle,
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

pub(super) fn push_node(
    scene: &mut SceneBuilder,
    id: NodeId,
    node: &Node,
    rect: Rect,
    paint: &NodePaint<'_>,
) {
    let style = paint.style;
    match &node.kind {
        NodeType::Container => push_container(scene, id, rect, paint),
        NodeType::Text { .. } => push_text(scene, id, node, rect, paint),
        NodeType::Image { source, fit } => {
            scene.draw_image(crate::renderer::scene_command::DrawImage {
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
            crate::renderer::color_glyph_paint::emit_color_glyphs(
                target,
                id,
                rect,
                paragraph,
                style.opacity,
            );
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

pub(super) const fn elevation_shadow(elevation: Elevation) -> Shadow {
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
