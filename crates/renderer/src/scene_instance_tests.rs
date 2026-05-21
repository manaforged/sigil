use hal::{LogicalSize, Point, Rect, Size};
use kernel::{Cx, NodeId, TextContent, prelude::*};
use layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use style::{NodeStyleExt, Rgba8};

use super::{
    RectInstance, collect_instances, draw_gradient_instance, draw_rect_instance,
    draw_shadow_instance, expanded_shadow_rect, instance_stride,
};
use crate::CornerRadii;
use crate::scene::emit_scene;
use crate::scene_command::{
    DrawGradient, DrawRect, GradientStop, LinearGradient, SceneGradientStyle, SceneRectStyle,
    Stroke, StrokeAlign, StrokeStyle,
};
use crate::shadow_paint::{Shadow, ShadowKind};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::default()
    }
}

fn lay_out(cx: &mut Cx, root: NodeId) {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("compute succeeds");
}

#[test]
fn collect_instances_drops_text_items() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(120.0, 48.0))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(60.0, 48.0))
                    .class("bg-accent"),
            )
            .child(text("dropped"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(60.0, 48.0))
                    .class("bg-accent"),
            ),
    );
    lay_out(&mut cx, root);
    #[allow(deprecated)]
    let scene = emit_scene(&cx, root).to_flat();
    let instances = collect_instances(&scene);
    assert_eq!(instances.len(), 2);
}

#[test]
fn rect_instance_carries_position_and_color() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .class("bg-accent"),
    );
    lay_out(&mut cx, root);
    #[allow(deprecated)]
    let scene = emit_scene(&cx, root).to_flat();
    let instances = collect_instances(&scene);
    assert_eq!(instances.len(), 1);
    let inst: &RectInstance = instances.first().expect("one instance");
    assert!((inst.rect[2] - 120.0).abs() < 1e-6);
    assert!((inst.rect[3] - 48.0).abs() < 1e-6);
    for ch in inst.color {
        assert!((0.0..=1.0).contains(&ch));
    }
    assert!((inst.params[0] - 1.0).abs() < 1e-6);
}

#[test]
fn instance_stride_matches_struct_size() {
    assert_eq!(
        instance_stride(),
        core::mem::size_of::<RectInstance>() as u64
    );
}

#[test]
fn draw_rect_instance_packs_solid_centered_stroke() {
    let stroke = stroke(StrokeStyle::Solid, StrokeAlign::Centered, 2.0);
    let inst = draw_rect_instance(&DrawRect::new(rect(), rect_style(stroke)), 1.0);

    assert_array_close(inst.stroke_params, [0.2, 0.4, 0.6, 0.8]);
    assert_array_close(inst.stroke_meta, [2.0, 0.0, 0.0, 0.0]);
}

#[test]
fn draw_rect_instance_packs_dashed_inside_stroke() {
    let stroke = stroke(StrokeStyle::Dashed(6.0), StrokeAlign::Inside, 3.0);
    let inst = draw_rect_instance(&DrawRect::new(rect(), rect_style(stroke)), 1.0);

    assert_array_close(inst.stroke_params, [0.2, 0.4, 0.6, 0.8]);
    assert_array_close(inst.stroke_meta, [3.0, 1.0, 1.0, 6.0]);
}

#[test]
fn draw_gradient_instance_packs_outside_stroke() {
    let stroke = stroke(StrokeStyle::Solid, StrokeAlign::Outside, 4.0);
    let draw = DrawGradient::new(rect(), gradient_style(stroke));
    let inst = draw_gradient_instance(&draw, 1.0, 0, 2);

    assert_array_close(inst.stroke_params, [0.2, 0.4, 0.6, 0.8]);
    assert_array_close(inst.stroke_meta, [4.0, 0.0, 2.0, 0.0]);
}

#[test]
fn draw_rect_instance_packs_corner_radii() {
    let style = SceneRectStyle::new(color(), CornerRadii::top(8.0), 10_000);
    let inst = draw_rect_instance(&DrawRect::new(rect(), style), 1.0);

    assert_array_close(inst.corner_radii, [8.0, 8.0, 0.0, 0.0]);
}

#[test]
fn draw_rect_instance_packs_uniform_corner_radii() {
    let style = SceneRectStyle::new(color(), CornerRadii::uniform(6.0), 10_000);
    let inst = draw_rect_instance(&DrawRect::new(rect(), style), 1.0);

    assert_array_close(inst.corner_radii, [6.0, 6.0, 6.0, 6.0]);
}

#[test]
fn draw_gradient_instance_packs_corner_radii() {
    let style = SceneGradientStyle::new(gradient(), CornerRadii::ZERO, 10_000);
    let inst = draw_gradient_instance(&DrawGradient::new(rect(), style), 1.0, 0, 2);

    assert_array_close(inst.corner_radii, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn draw_rect_instance_packs_zero_stroke_when_none() {
    let style = SceneRectStyle::new(color(), CornerRadii::ZERO, 10_000);
    let inst = draw_rect_instance(&DrawRect::new(rect(), style), 1.0);

    assert_array_close(inst.stroke_params, [0.0; 4]);
    assert_array_close(inst.stroke_meta, [0.0; 4]);
}

#[test]
fn shadow_instance_expands_by_three_blurs_plus_abs_spread_then_offsets() {
    let shadow = Shadow::new([6.0, -2.0], 8.0, -3.0, [0.0, 0.0, 0.0, 0.35]);
    let expanded = expanded_shadow_rect(rect(), shadow);

    assert_eq!(expanded, Rect::new(Point::new(-21.0, -29.0), 102.0, 102.0));
}

#[test]
fn inner_shadow_expansion_stays_on_surface_rect() {
    let shadow = Shadow::new([6.0, -2.0], 8.0, 3.0, [0.0, 0.0, 0.0, 0.35])
        .with_kind(ShadowKind::Inner);
    let expanded = expanded_shadow_rect(rect(), shadow);

    assert_eq!(expanded, rect());
}

#[test]
fn draw_shadow_instance_carries_surface_and_expanded_rects() {
    let shadow = Shadow::new([2.0, 3.0], 4.0, 1.0, [0.1, 0.2, 0.3, 0.4]);
    let inst = draw_shadow_instance(rect(), CornerRadii::top(5.0), shadow, 0.5);

    assert_array_close(inst.rect, [-11.0, -10.0, 74.0, 74.0]);
    assert_array_close(inst.surface_rect, [0.0, 0.0, 48.0, 48.0]);
    assert_array_close(inst.corner_radii, [5.0, 5.0, 0.0, 0.0]);
    assert_array_close(inst.color, [0.1, 0.2, 0.3, 0.2]);
    assert_array_close(inst.shadow_params, [2.0, 3.0, 4.0, 1.0]);
    assert_array_close(inst.shadow_meta, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn draw_inner_shadow_instance_encodes_kind_and_surface_quad() {
    let shadow = Shadow::new([2.0, 3.0], 4.0, 1.0, [0.1, 0.2, 0.3, 0.4])
        .with_kind(ShadowKind::Inner);
    let inst = draw_shadow_instance(rect(), CornerRadii::top(5.0), shadow, 0.5);

    assert_array_close(inst.rect, [0.0, 0.0, 48.0, 48.0]);
    assert_array_close(inst.surface_rect, [0.0, 0.0, 48.0, 48.0]);
    assert_array_close(inst.shadow_meta, [1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn inner_shadow_coverage_is_inside_only() {
    let inside_alpha = inner_shadow_alpha_for_test(Point::new(2.0, 24.0), rect(), 4.0, 0.5);
    let outside_alpha = inner_shadow_alpha_for_test(Point::new(-1.0, 24.0), rect(), 4.0, 0.5);

    assert!(inside_alpha > 0.0);
    assert!(outside_alpha.abs() < 1e-6);
}

fn assert_array_close(actual_values: [f32; 4], expected_values: [f32; 4]) {
    for (actual, expected) in actual_values.into_iter().zip(expected_values) {
        assert!((actual - expected).abs() < 1e-6);
    }
}

fn inner_shadow_alpha_for_test(sample: Point, surface: Rect, spread: f32, color_alpha: f32) -> f32 {
    if sample.x < surface.origin.x
        || sample.y < surface.origin.y
        || sample.x > surface.origin.x + surface.width
        || sample.y > surface.origin.y + surface.height
    {
        return 0.0;
    }
    let inner_left = surface.origin.x + spread;
    let inner_right = surface.origin.x + surface.width - spread;
    let inner_top = surface.origin.y + spread;
    let inner_bottom = surface.origin.y + surface.height - spread;
    let inside_inner = sample.x >= inner_left
        && sample.x <= inner_right
        && sample.y >= inner_top
        && sample.y <= inner_bottom;
    color_alpha * if inside_inner { 0.0 } else { 1.0 }
}

fn rect_style(stroke: Stroke) -> SceneRectStyle {
    SceneRectStyle::new(color(), CornerRadii::ZERO, 10_000).with_stroke(stroke)
}

fn gradient_style(stroke: Stroke) -> SceneGradientStyle {
    SceneGradientStyle::new(gradient(), CornerRadii::ZERO, 10_000).with_stroke(stroke)
}

const fn stroke(style: StrokeStyle, align: StrokeAlign, width: f32) -> Stroke {
    Stroke {
        color: [0.2, 0.4, 0.6, 0.8],
        width,
        style,
        align,
    }
}

fn gradient() -> LinearGradient {
    LinearGradient::new(
        Point::new(0.0, 0.0),
        Point::new(48.0, 48.0),
        smallvec::smallvec![
            GradientStop::new(0.0, color()),
            GradientStop::new(
                1.0,
                Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            ),
        ],
    )
}

const fn color() -> Rgba8 {
    Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    }
}

const fn rect() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 48.0, 48.0)
}
