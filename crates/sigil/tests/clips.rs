#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small"
)]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::{ImageSource, container, image};
use sigil::layout::{LayoutStyle, Overflow};
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{Clip, CommandScene, DrawRect, SceneBuilder, SceneCommand};
use sigil::style::{Color, Paint, Radius};
use sigil::widgets::canvas;
use sigil_kittest::Harness;
use sigil_kittest::gpu::{headless_renderer, render_scene_to_image};
use sigil_kittest::parity::{Frame, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, cpu_rgba, diff};

#[path = "clips/perf.rs"]
mod perf;

const SIDE: u32 = 60;
const SCALE: f32 = 2.0;
const RED: Rgba8 = Rgba8 {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

const fn whole() -> Rect {
    Rect::new(Point::new(0.0, 0.0), SIDE as f32, SIDE as f32)
}

fn sized(node: impl IntoNode + 'static) -> impl IntoNode {
    container()
        .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
        .child(node)
}

fn clipped(clips: Vec<Clip>) -> impl IntoNode {
    let paint = canvas(move |scene: &mut SceneBuilder, rect: Rect| {
        nest(scene, &clips, rect);
    })
    .width(SIDE as f32)
    .height(SIDE as f32);
    sized(paint)
}

fn nest(scene: &mut SceneBuilder, clips: &[Clip], rect: Rect) {
    match clips.split_first() {
        Some((first, inner)) => scene.with_clip(first.clone(), |scene| nest(scene, inner, rect)),
        None => scene.draw_rect(DrawRect::new(rect, RED)),
    }
}

fn red_image(side: u32) -> ImageSource {
    let pixels = [RED.r, RED.g, RED.b, RED.a].repeat((side * side) as usize);
    ImageSource::from_rgba8(side, side, pixels).expect("red image")
}

fn avatar() -> impl IntoNode {
    let child = image(red_image(40)).layout(LayoutStyle::block().fixed_size(40.0, 40.0));
    let parent = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(40.0, 40.0)
                .overflow_x(Overflow::Hidden)
                .overflow_y(Overflow::Hidden),
        )
        .rounded(Radius::Full)
        .child(child);
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SIDE as f32, SIDE as f32)
                .padding(Space::raw(10.0, "test fixture")),
        )
        .child(parent)
}

#[track_caller]
fn gpu_frame(ui: &mut Harness) -> image::RgbaImage {
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no GPU adapter: the harness returned no frame"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

fn cpu_frame(scene: &CommandScene) -> image::RgbaImage {
    let device = (SIDE as f32 * SCALE) as u32;
    let frame = Frame {
        scene,
        viewport: (SIDE as f32, SIDE as f32),
        size: PhysicalSize::new(device, device),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    image::RgbaImage::from_raw(device, device, rgba).expect("cpu frame")
}

fn frames<N: IntoNode + 'static>(app: fn() -> N) -> Vec<(&'static str, image::RgbaImage)> {
    let mut pixels = Harness::new(app).pixels(SIDE, SIDE).scale(SCALE);
    let mut surface = Harness::new(app).gpu(SIDE, SIDE, SCALE);
    let scene = surface.host().emit((SIDE as f32, SIDE as f32));
    vec![
        ("pixels", gpu_frame(&mut pixels)),
        ("gpu", gpu_frame(&mut surface)),
        ("cpu", cpu_frame(&scene)),
    ]
}

fn distance(a: image::Rgba<u8>, b: image::Rgba<u8>) -> u32 {
    a.0.iter()
        .zip(b.0)
        .take(3)
        .map(|(x, y)| u32::from(x.abs_diff(y)))
        .sum()
}

fn coverage(frame: &image::RgbaImage, (x, y): (f32, f32)) -> f32 {
    let background = *frame.get_pixel(0, 0);
    let ink = image::Rgba([RED.r, RED.g, RED.b, RED.a]);
    let probe = *frame.get_pixel((x * SCALE) as u32, (y * SCALE) as u32);
    distance(probe, background) as f32 / distance(ink, background).max(1) as f32
}

#[track_caller]
fn assert_probes<N: IntoNode + 'static>(app: fn() -> N, lit: &[(f32, f32)], clear: &[(f32, f32)]) {
    for (name, frame) in frames(app) {
        for &point in lit {
            let found = coverage(&frame, point);
            assert!(
                found > 0.9,
                "{name}: {point:?} keeps the child (coverage {found:.2})"
            );
        }
        for &point in clear {
            let found = coverage(&frame, point);
            assert!(
                found < 0.1,
                "{name}: {point:?} is clipped away (coverage {found:.2})"
            );
        }
    }
}

#[test]
fn an_image_under_a_rounded_overflow_parent_is_clear_outside_the_corner_arc() {
    assert_probes(avatar, &[(17.0, 17.0), (30.0, 30.0)], &[(13.0, 13.0)]);
}

fn circle() -> impl IntoNode {
    clipped(vec![Clip::circle(Point::new(30.0, 30.0), 20.0)])
}

#[test]
fn a_circle_clip_keeps_only_the_disc() {
    let lit = [(30.0, 30.0), (30.0, 48.0), (12.0, 30.0)];
    let clear = [(13.0, 13.0), (47.0, 47.0), (30.0, 6.0)];
    assert_probes(circle, &lit, &clear);
}

fn triangle() -> impl IntoNode {
    let points = [
        Point::new(30.0, 5.0),
        Point::new(55.0, 55.0),
        Point::new(5.0, 55.0),
    ];
    clipped(vec![Clip::polygon(&points)])
}

#[test]
fn a_polygon_clip_keeps_only_the_triangle() {
    let lit = [(30.0, 40.0), (30.0, 12.0), (10.0, 52.0)];
    let clear = [(10.0, 12.0), (50.0, 12.0), (30.0, 58.0)];
    assert_probes(triangle, &lit, &clear);
}

fn ellipse() -> impl IntoNode {
    clipped(vec![Clip::ellipse(Rect::new(
        Point::new(5.0, 20.0),
        50.0,
        20.0,
    ))])
}

#[test]
fn an_ellipse_clip_keeps_only_the_ellipse() {
    let lit = [(30.0, 30.0), (8.0, 30.0), (30.0, 22.0)];
    let clear = [(30.0, 17.0), (8.0, 22.0), (52.0, 38.0)];
    assert_probes(ellipse, &lit, &clear);
}

fn lens() -> impl IntoNode {
    clipped(vec![
        Clip::circle(Point::new(24.0, 30.0), 16.0),
        Clip::circle(Point::new(36.0, 30.0), 16.0),
    ])
}

#[test]
fn nested_rounded_clips_keep_only_their_intersection() {
    let lit = [(30.0, 30.0), (30.0, 20.0)];
    let clear = [(12.0, 30.0), (48.0, 30.0), (22.0, 16.0), (38.0, 44.0)];
    assert_probes(lens, &lit, &clear);
}

fn disc_in_triangle() -> impl IntoNode {
    let points = [
        Point::new(30.0, 0.0),
        Point::new(60.0, 60.0),
        Point::new(0.0, 60.0),
    ];
    clipped(vec![
        Clip::polygon(&points),
        Clip::circle(Point::new(30.0, 30.0), 20.0),
    ])
}

#[test]
fn a_disc_inside_a_polygon_keeps_only_their_intersection() {
    let lit = [(30.0, 30.0), (30.0, 48.0)];
    let clear = [(18.0, 16.0), (42.0, 16.0), (13.0, 13.0)];
    assert_probes(disc_in_triangle, &lit, &clear);
}

fn layers(clip: Clip) -> (usize, usize, usize) {
    let mut builder = SceneBuilder::new();
    builder.with_clip(clip, |scene| scene.draw_rect(DrawRect::new(whole(), RED)));
    let scene = builder.finish();
    let pushed = scene
        .commands
        .iter()
        .filter(|command| {
            matches!(command, SceneCommand::PushLayer(_))
                || matches!(command, SceneCommand::PushClip(pushed) if !matches!(pushed, Clip::Rect(_)))
        })
        .count();
    let viewport = (SIDE as f32, SIDE as f32);
    let device = (SIDE as f32 * SCALE) as u32;
    let mut cpu = CpuRenderer::new();
    let frame = Frame {
        scene: &scene,
        viewport,
        size: PhysicalSize::new(device, device),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    cpu_rgba(&mut cpu, &frame).expect("cpu render");
    let mut gpu = headless_renderer().expect("renderer").expect("GPU adapter");
    render_scene_to_image(&mut gpu, &scene, viewport, [device, device]).expect("gpu render");
    (
        pushed,
        cpu.last_frame_stats().offscreen_layers,
        gpu.last_frame_stats().offscreen_layers,
    )
}

#[test]
fn a_rect_clip_draws_through_the_scissor_without_a_layer() {
    let rect = Rect::new(Point::new(10.0, 10.0), 40.0, 40.0);
    assert_eq!(
        layers(Clip::Rect(rect)),
        (0, 0, 0),
        "(composited clips, cpu layers, gpu layers)"
    );
    assert_eq!(
        layers(Clip::circle(Point::new(30.0, 30.0), 20.0)),
        (1, 1, 1),
        "a circle clip composites one layer"
    );
    assert_eq!(
        layers(Clip::polygon(&[
            Point::new(30.0, 5.0),
            Point::new(55.0, 55.0),
            Point::new(5.0, 55.0)
        ])),
        (1, 1, 1),
        "a polygon clip composites one layer"
    );
}

fn gallery() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(SIDE as f32 * 4.0, SIDE as f32))
        .child(avatar())
        .child(circle())
        .child(triangle())
        .child(lens())
}

#[test]
fn clipped_scenes_agree_on_the_cpu_and_the_gpu() {
    let (width, height) = (SIDE * 4, SIDE);
    let mut ui = Harness::new(gallery).gpu(width, height, SCALE);
    let gpu = gpu_frame(&mut ui);
    let viewport = (width as f32, height as f32);
    let scene = ui.host().emit(viewport);
    let frame = Frame {
        scene: &scene,
        viewport,
        size: PhysicalSize::new(gpu.width(), gpu.height()),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    let measured = diff(gpu.as_raw(), &cpu);
    println!(
        "clips cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    assert!(
        measured.mean <= MAX_MEAN_DIFF && measured.close >= MIN_CLOSE_FRACTION,
        "cpu and gpu differ: {measured:?}, limits mean {MAX_MEAN_DIFF} close {MIN_CLOSE_FRACTION}"
    );
}

fn clip_path_disc() -> impl IntoNode {
    container()
        .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
        .bg(RED)
        .clip_path(sigil::style::ClipShape::Circle(Some(20)))
}

#[test]
fn a_circle_clip_path_keeps_only_the_disc_of_its_own_background() {
    let lit = [(30.0, 30.0), (30.0, 48.0), (12.0, 30.0)];
    let clear = [(13.0, 13.0), (47.0, 47.0), (30.0, 6.0)];
    assert_probes(clip_path_disc, &lit, &clear);
}

#[test]
fn a_click_outside_the_clip_path_disc_misses_the_node() {
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let seen = clicks.clone();
    let mut ui = Harness::new(move || {
        let seen = seen.clone();
        container()
            .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
            .child(
                button("disc")
                    .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
                    .clip_path(sigil::style::ClipShape::Circle(Some(20)))
                    .activate(move || seen.set(seen.get() + 1)),
            )
    })
    .pixels(SIDE, SIDE);
    ui.click(&Point::new(6.0, 6.0));
    assert_eq!(
        clicks.get(),
        0,
        "a click in the box corner, outside the 20 px disc, misses"
    );
    ui.click(&Point::new(30.0, 30.0));
    assert_eq!(clicks.get(), 1, "a click at the disc centre hits");
}
