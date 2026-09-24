use crate::hal::{Point, Rect};
use crate::kernel::{ImageCrop, Node, NodeId, NodeType, TextContent, ThemedImage};
use crate::scene::builder::SceneBuilder;
use crate::scene::command::{
    CellBorder, CellRect, DecorationKind, DrawImage, GlyphRun, SceneDecorationStyle,
    SceneRectStyle, Stroke, StrokeAlign, StrokeStyle, TextFlow, shape_run,
};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;
use crate::scene::style_adapter::{corner_radii_for, focus_ring_for, text_style_for};
use crate::style::{Elevation, RenderStyle, Rgba8, TextDecoration, TextStyle, Theme, Tone};

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
        CellRect {
            outline: true,
            ..CellRect::default()
        },
    );
}

pub(super) struct NodePaint<'a> {
    pub(super) style: &'a RenderStyle,
    pub(super) theme: &'a Theme,
    pub(super) text_base: &'a TextStyle,
    pub(super) tone: Option<Tone>,
    pub(super) leaf: bool,
}

impl NodePaint<'_> {
    fn cell(&self) -> CellRect {
        CellRect {
            tone: self.style.background_color.map_or(self.tone, |_| None),
            leaf: self.leaf,
            border: None,
            outline: false,
        }
    }

    fn border(&self, rect: Rect) -> Option<CellBorder> {
        let widths = border_edges(self.style).map(|width| width.max(0.0));
        if widths.iter().all(|width| *width <= 0.0) {
            return None;
        }
        Some(CellBorder {
            rect,
            widths,
            color: border_color(self.style, self.theme),
            rounded: self.style.border_radius.is_some_and(|radius| radius.0 > 0),
        })
    }
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
    if style.background_color.is_some() || stroke.is_some() || paint.tone.is_some() {
        let fill = style.background_color.unwrap_or(Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
        let mut rect_style =
            SceneRectStyle::new(fill, corner_radii_for(style.border_radius), style.opacity);
        let mut cell = paint.cell();
        if let Some(stroke) = stroke {
            rect_style = rect_style.with_stroke(stroke);
            cell.border = paint.border(rect);
        }
        scene.draw_rect_for_node(id, rect, rect_style, cell);
    }
    push_side_borders(scene, id, rect, paint);
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
            let source = node
                .get_extension::<ThemedImage>()
                .and_then(|themed| themed.resolve(paint.theme))
                .unwrap_or_else(|| source.clone());
            scene.draw_image(DrawImage {
                node: Some(id),
                source,
                fit: *fit,
                destination: rect,
                opacity: style.opacity,
                crop: node.get_extension::<ImageCrop>().copied(),
            });
        }
        NodeType::Fragment => {}
    }
}

fn push_text(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    let NodeType::Text {
        content,
        wrap,
        ellipsis,
    } = &node.kind
    else {
        return;
    };
    let Some(color) = style.text_color else {
        return;
    };
    let flow = match (*wrap, *ellipsis) {
        (true, _) => TextFlow::Wrap,
        (false, true) => TextFlow::Ellipsis,
        (false, false) => TextFlow::Line,
    };
    let mut text_style = text_style_for(node, paint.theme, paint.text_base);
    text_style.runs = node.get_extension::<crate::style::TextRuns>().cloned();
    let run = shape_run(
        GlyphRun::new(
            rect,
            text_content(content),
            color,
            text_style.clone(),
            style.opacity,
        )
        .with_cell(flow, paint.tone)
        .with_inks(super::runs::inks(text_style.runs.as_ref(), paint.theme)),
    );
    let run = super::runs::split(scene, id, run);
    let overflows = *ellipsis && overflows(&run, rect);
    let emit = |target: &mut SceneBuilder| {
        push_color_glyphs(target, id, &run, style.opacity);
        target.draw_glyph_run_for_node(id, run);
    };
    if overflows {
        scene.with_clip(rect, emit);
    } else {
        emit(scene);
    }
    push_text_decoration(scene, rect, color, style.opacity, &text_style);
}

#[cfg(feature = "scene")]
fn overflows(run: &GlyphRun, rect: Rect) -> bool {
    use crate::hal::ParagraphMetrics as _;
    run.paragraph().is_some_and(|paragraph| {
        paragraph.width() > rect.width + 0.5 || paragraph.height() > rect.height + 0.5
    })
}

#[cfg(not(feature = "scene"))]
const fn overflows(_: &GlyphRun, _: Rect) -> bool {
    false
}

#[cfg(feature = "scene")]
fn push_color_glyphs(scene: &mut SceneBuilder, id: NodeId, run: &GlyphRun, opacity: u16) {
    if let Some(paragraph) = run.paragraph() {
        crate::renderer::color_glyph_paint::emit_color_glyphs(
            scene, id, run.rect, paragraph, opacity,
        );
    }
}

#[cfg(not(feature = "scene"))]
const fn push_color_glyphs(_: &mut SceneBuilder, _: NodeId, _: &GlyphRun, _: u16) {}

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

fn push_side_borders(scene: &mut SceneBuilder, id: NodeId, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    let theme = paint.theme;
    let [top, right, bottom, left] = border_edges(style);
    if top <= 0.0 && right <= 0.0 && bottom <= 0.0 && left <= 0.0 {
        return;
    }
    if top > 0.0 && borders_uniform([top, right, bottom, left]) {
        return;
    }
    let color = border_color(style, theme);
    let fill = SceneRectStyle::new(color, CornerRadii::new(0.0, 0.0, 0.0, 0.0), style.opacity);
    let bottom = bottom.min(rect.height);
    let right = right.min(rect.width);
    let sides = [
        (
            top,
            Rect::new(rect.origin, rect.width, top.min(rect.height)),
        ),
        (
            bottom,
            Rect::new(
                Point::new(rect.origin.x, rect.origin.y + rect.height - bottom),
                rect.width,
                bottom,
            ),
        ),
        (
            left,
            Rect::new(rect.origin, left.min(rect.width), rect.height),
        ),
        (
            right,
            Rect::new(
                Point::new(rect.origin.x + rect.width - right, rect.origin.y),
                right,
                rect.height,
            ),
        ),
    ];
    let mut cell = CellRect {
        border: paint.border(rect),
        outline: true,
        ..paint.cell()
    };
    for (width, side) in sides {
        if width > 0.0 {
            scene.draw_rect_for_node(id, side, fill, cell);
            cell.border = None;
        }
    }
}

fn text_content(content: &TextContent) -> String {
    content.resolve()
}
