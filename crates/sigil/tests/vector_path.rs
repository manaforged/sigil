#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]
#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small"
)]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{CommandScene, SceneBuilder, SceneCommand, StrokeCap, StrokeJoin};
use sigil::style::Rgba;
use sigil::widgets::{PathNode, canvas, path};
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, cpu_rgba, diff};

#[path = "vector_path/blend.rs"]
mod blend;
#[path = "vector_path/dash.rs"]
mod dash;
#[path = "vector_path/placement.rs"]
mod placement;
#[path = "vector_path/retained.rs"]
mod retained;

const SIDE: u32 = 60;
const SCALE: f32 = 2.0;
const MAX_EDGE_PX: usize = 2;
const INK: Color = Color {
    r: 255,
    g: 40,
    b: 40,
    a: 255,
};

#[track_caller]
fn frame_of(ui: &mut Harness) -> image::RgbaImage {
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no GPU adapter: the harness returned no frame"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

fn distance(a: image::Rgba<u8>, b: image::Rgba<u8>) -> u32 {
    a.0.iter()
        .zip(b.0)
        .take(3)
        .map(|(x, y)| u32::from(x.abs_diff(y)))
        .sum()
}

fn coverage(frame: &image::RgbaImage, x: u32, y: u32) -> f32 {
    let background = *frame.get_pixel(0, 0);
    let ink = image::Rgba([INK.r, INK.g, INK.b, INK.a]);
    distance(*frame.get_pixel(x, y), background) as f32 / distance(ink, background) as f32
}

fn at_logical(frame: &image::RgbaImage, x: f32, y: f32, scale: f32) -> f32 {
    coverage(frame, (x * scale) as u32, (y * scale) as u32)
}

fn edge_widths(frame: &image::RgbaImage, y: u32) -> (usize, usize) {
    let row: Vec<f32> = (0..frame.width()).map(|x| coverage(frame, x, y)).collect();
    let peak = row
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(at, _)| at);
    let partial = |c: &&f32| **c > 0.1 && **c < 0.9;
    let left = row[..peak].iter().filter(partial).count();
    let right = row[peak + 1..].iter().filter(partial).count();
    (left, right)
}

fn widest_edge(frame: &image::RgbaImage, rows: std::ops::Range<u32>) -> usize {
    rows.map(|y| {
        let (left, right) = edge_widths(frame, y);
        left.max(right)
    })
    .max()
    .unwrap_or(0)
}

fn diagonal() -> PathNode {
    path(SIDE, SIDE)
        .move_to(10.0, 10.0)
        .line_to(50.0, 50.0)
        .stroke(INK, 1.0)
}

fn small_diagonal() -> PathNode {
    path(15, 15)
        .move_to(2.5, 2.5)
        .line_to(12.5, 12.5)
        .stroke(INK, 1.0)
}

fn diagonal_canvas() -> impl IntoNode {
    canvas(|scene: &mut SceneBuilder, rect: Rect| diagonal().draw(scene, rect, &Theme::light()))
        .width(SIDE as f32)
        .height(SIDE as f32)
}

fn fitted_diagonal() -> impl IntoNode {
    small_diagonal().fit_size(SIDE as f32)
}

fn zoomed_diagonal() -> impl IntoNode {
    canvas(|scene: &mut SceneBuilder, _rect: Rect| {
        let zoom = [[4.0, 0.0, 0.0], [0.0, 4.0, 0.0]];
        scene.with_transform(zoom, |scene| {
            let destination = Rect::new(Point::new(0.0, 0.0), 15.0, 15.0);
            small_diagonal().draw(scene, destination, &Theme::light());
        });
    })
    .width(SIDE as f32)
    .height(SIDE as f32)
}

fn weakest_peak(frame: &image::RgbaImage, rows: std::ops::Range<u32>) -> f32 {
    rows.map(|y| {
        (0..frame.width())
            .map(|x| coverage(frame, x, y))
            .fold(0.0, f32::max)
    })
    .fold(1.0, f32::min)
}

fn split_block_rows(frame: &image::RgbaImage, rows: std::ops::Range<u32>) -> usize {
    let split = |x: u32, y: u32| {
        let first = *frame.get_pixel(x, y);
        [(1, 0), (0, 1), (1, 1)]
            .iter()
            .any(|(dx, dy)| distance(*frame.get_pixel(x + dx, y + dy), first) > 24)
    };
    rows.step_by(2)
        .filter(|&y| (0..frame.width() - 1).step_by(2).any(|x| split(x, y)))
        .count()
}

#[track_caller]
fn assert_sharp(ui: &mut Harness, what: &str) {
    let frame = frame_of(ui);
    let device = (SIDE as f32 * SCALE) as u32;
    assert_eq!(frame.dimensions(), (device, device), "{what}: device size");
    let (widest, peak) = (widest_edge(&frame, 30..90), weakest_peak(&frame, 30..90));
    let split = split_block_rows(&frame, 30..90);
    println!(
        "{what}: widest edge {widest} px, weakest row peak {peak:.2}, split block rows {split}/30"
    );
    assert!(
        widest <= MAX_EDGE_PX,
        "{what}: an edge spans {widest} device px; a device-resolution path spans at most {MAX_EDGE_PX}"
    );
    assert!(
        peak >= 0.95,
        "{what}: a row has no fully covered pixel (peak {peak:.2}); a 2.83 px chord always holds one"
    );
    assert_eq!(
        split, 30,
        "{what}: the edge falls inside 2x2 device blocks on every block row; an upscaled 1x bitmap has none"
    );
}

fn retina(app: impl IntoNode + 'static) -> Harness {
    Harness::new(app).pixels(SIDE, SIDE).scale(SCALE)
}

#[test]
fn a_one_px_diagonal_at_scale_two_has_device_sharp_edges() {
    let mut ui = retina(diagonal_canvas);
    assert_sharp(&mut ui, "pixels 2x");
}

#[test]
fn a_one_px_diagonal_on_the_gpu_surface_has_device_sharp_edges() {
    let mut ui = Harness::new(diagonal_canvas).gpu(SIDE, SIDE, SCALE);
    assert_sharp(&mut ui, "gpu 2x");
}

#[test]
fn a_path_fitted_four_times_larger_stays_sharp() {
    let mut ui = retina(fitted_diagonal);
    assert_sharp(&mut ui, "fit_size 4x");
}

#[test]
fn a_path_under_a_four_times_transform_stays_sharp() {
    let mut ui = retina(zoomed_diagonal);
    assert_sharp(&mut ui, "transform 4x pixels");
}

#[test]
fn a_path_under_a_four_times_transform_stays_sharp_on_the_gpu() {
    let mut ui = Harness::new(zoomed_diagonal).gpu(SIDE, SIDE, SCALE);
    assert_sharp(&mut ui, "transform 4x gpu");
}

fn star(node: PathNode) -> PathNode {
    let point = |k: usize| {
        let angle = 144.0f32.mul_add(k as f32, -90.0).to_radians();
        (
            25.0f32.mul_add(angle.cos(), 30.0),
            25.0f32.mul_add(angle.sin(), 30.0),
        )
    };
    let (x, y) = point(0);
    let node = node.move_to(x, y);
    (1..5)
        .fold(node, |node, k| {
            let (next_x, next_y) = point(k);
            node.line_to(next_x, next_y)
        })
        .close()
        .fill(INK)
}

fn shape_canvas(shape: fn() -> PathNode) -> impl IntoNode {
    canvas(move |scene: &mut SceneBuilder, rect: Rect| {
        shape().draw(scene, rect, &Theme::light());
    })
    .width(80.0)
    .height(80.0)
}

fn probes(shape: fn() -> PathNode, points: &[(f32, f32)]) -> Vec<f32> {
    let mut ui = Harness::new(shape_canvas(shape))
        .pixels(80, 80)
        .scale(SCALE);
    let frame = frame_of(&mut ui);
    points
        .iter()
        .map(|&(x, y)| at_logical(&frame, x, y, SCALE))
        .collect()
}

#[test]
fn a_nonzero_star_fills_its_centre() {
    let star_node = || star(path(80, 80));
    let found = probes(star_node, &[(30.0, 30.0), (30.0, 12.0)]);
    assert_lit(found[1], "the nonzero star's top arm");
    assert_lit(found[0], "the nonzero star's centre, winding 2,");
}

#[track_caller]
fn assert_lit(found: f32, what: &str) {
    assert!(found > 0.9, "{what} is lit (coverage {found:.2})");
}

#[track_caller]
fn assert_empty(found: f32, what: &str) {
    assert!(found < 0.1, "{what} is empty (coverage {found:.2})");
}

fn path_scene() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_row())
        .child(shape_canvas(|| star(path(80, 80))))
        .child(small_diagonal().fit_size(40.0))
}

#[test]
fn a_path_scene_agrees_on_the_cpu_and_the_gpu() {
    let mut ui = Harness::new(path_scene).gpu(120, 80, SCALE);
    let gpu = frame_of(&mut ui);
    let viewport = (120.0, 80.0);
    let scene = ui.host().emit(viewport);
    let frame = Frame {
        scene: &scene,
        viewport,
        size: PhysicalSize::new(240, 160),
        clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
    };
    let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    assert_lit(coverage(&gpu, 60, 60), "the gpu frame's star centre");
    let measured = diff(gpu.as_raw(), &cpu);
    println!(
        "cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    assert!(
        measured.mean <= MAX_MEAN_DIFF && measured.close >= MIN_CLOSE_FRACTION,
        "cpu and gpu differ: {measured:?}, limits mean {MAX_MEAN_DIFF} close {MIN_CLOSE_FRACTION}"
    );
}

#[test]
fn a_path_node_emits_vector_commands_not_a_bitmap() {
    let mut ui = Harness::new(path_scene).pixels(120, 80).scale(SCALE);
    let scene: CommandScene = ui.host().emit((120.0, 80.0));
    let bitmaps: Vec<&SceneCommand> = scene
        .commands
        .iter()
        .filter(|command| matches!(command, SceneCommand::Image(_) | SceneCommand::Sprite(_)))
        .collect();
    assert!(
        bitmaps.is_empty(),
        "a star canvas and a path() node emit {} Image/Sprite commands: {:?}",
        bitmaps.len(),
        bitmaps.first()
    );
}

#[test]
fn an_even_odd_star_leaves_its_centre_empty() {
    let star_node = || star(path(80, 80)).even_odd();
    let found = probes(star_node, &[(30.0, 30.0), (30.0, 12.0)]);
    assert_lit(found[1], "the even-odd star's top arm");
    assert_empty(found[0], "the even-odd star's centre");
}

fn capped(cap: StrokeCap) -> PathNode {
    path(80, 80)
        .move_to(20.0, 20.0)
        .line_to(50.0, 20.0)
        .stroke(INK, 10.0)
        .cap(cap)
}

#[test]
fn a_square_cap_extends_the_stroke_by_half_its_width() {
    let square = probes(|| capped(StrokeCap::Square), &[(53.0, 20.0), (58.0, 20.0)]);
    assert_lit(square[0], "3 px past the end, inside the 5 px square cap,");
    assert_empty(square[1], "8 px past the end, beyond the 5 px square cap,");
    let butt = probes(|| capped(StrokeCap::Butt), &[(53.0, 20.0)]);
    assert_empty(butt[0], "3 px past the end of a butt cap");
}

fn joined(join: StrokeJoin) -> PathNode {
    path(80, 80)
        .move_to(10.0, 50.0)
        .line_to(50.0, 50.0)
        .line_to(50.0, 10.0)
        .stroke(INK, 20.0)
        .join(join)
}

const INSIDE_ROUND: (f32, f32) = (56.0, 56.0);
const MITER_TIP: (f32, f32) = (59.0, 59.0);

#[test]
fn a_round_join_covers_the_corner_at_half_the_width() {
    let round = probes(|| joined(StrokeJoin::Round), &[INSIDE_ROUND, MITER_TIP]);
    assert_lit(
        round[0],
        "a point 8.5 px out from the corner, inside the 10 px round join,",
    );
    assert_empty(
        round[1],
        "the miter tip, 12.7 px out, beyond the 10 px round join,",
    );
    let bevel = probes(|| joined(StrokeJoin::Bevel), &[INSIDE_ROUND]);
    assert_empty(bevel[0], "the same point beyond a bevel join's cut");
}

#[test]
fn a_miter_join_reaches_the_miter_tip() {
    let miter = probes(|| joined(StrokeJoin::Miter), &[MITER_TIP]);
    assert_lit(miter[0], "a point 1 px inside the miter tip at (60, 60)");
}
