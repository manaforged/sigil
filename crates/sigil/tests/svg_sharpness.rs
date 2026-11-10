#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small and positive"
)]

use sigil::geometry::PhysicalSize;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use sigil::scene::paint_backdrop;
use sigil::style::{Color, NumericAtomKind, Transform2D};
use sigil_kittest::Harness;

const VIEW: (u32, u32) = (480, 360);
const ZOOM: f32 = 4.0;
const DEEP_ZOOM: f32 = 16.0;
const MAX_EDGE: f32 = 2.0;
const MIN_STEP: f32 = 24.0;
const MIN_EDGES: usize = 8;

#[derive(Clone, Copy, Debug)]
enum Backend {
    Cpu,
    Gpu,
}

#[derive(Clone, Copy, Debug)]
enum Diagram {
    Mermaid,
    Graphviz,
    Math,
    Chain,
    Chart,
}

impl Diagram {
    fn node(self) -> NodeBuilder {
        match self {
            Self::Mermaid => mermaid(signal("flowchart LR\n  A[Start] --> B[End]\n".to_string())),
            Self::Graphviz => graphviz(signal("digraph { a -> b; a -> c; }".to_string())),
            Self::Math => math(signal(r"x^2 + \frac{a}{b}".to_string())),
            Self::Chain => mermaid(signal(
                "flowchart LR\n  A --> B --> C --> D --> E --> F --> G --> H\n".to_string(),
            )),
            Self::Chart => container().child(
                chart(vec![1.0, 6.0, 2.0, 8.0, 3.0, 7.0])
                    .line()
                    .width(VIEW.0 as f32)
                    .height(VIEW.1 as f32),
            ),
        }
    }
}

fn zoomed(diagram: Diagram, zoom: f32) -> NodeBuilder {
    container()
        .background(Token::Surface)
        .layout(
            LayoutStyle::block()
                .width(VIEW.0 as f32)
                .height(VIEW.1 as f32),
        )
        .child(
            container()
                .layout(LayoutStyle::block().width(VIEW.0 as f32))
                .transform(Transform2D::IDENTITY.origin(0.5, 0.0))
                .drive(NumericAtomKind::ScaleX, signal(zoom))
                .drive(NumericAtomKind::ScaleY, signal(zoom))
                .child(diagram.node()),
        )
}

fn capture(ui: &mut Harness, backend: Backend, scale: f32) -> image::RgbaImage {
    match backend {
        Backend::Gpu => ui.image().expect("render").expect("gpu"),
        Backend::Cpu => {
            let viewport = (VIEW.0 as f32, VIEW.1 as f32);
            let host = ui.host();
            let mut scene = host.emit(viewport);
            paint_backdrop(&mut scene, host.theme(), viewport);
            let target = ImageTarget {
                size: PhysicalSize::new((viewport.0 * scale) as u32, (viewport.1 * scale) as u32),
                viewport,
                clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
            };
            render_to_image(ImageRenderer::Cpu(&mut CpuRenderer::new()), &scene, target)
                .expect("cpu render")
        }
    }
}

fn luma(image: &image::RgbaImage, x: u32, y: u32) -> f32 {
    let [red, green, blue, _] = image.get_pixel(x, y).0;
    0.2126f32.mul_add(
        f32::from(red),
        0.7152f32.mul_add(f32::from(green), 0.0722 * f32::from(blue)),
    )
}

fn ramp(row: &[f32], start: usize) -> (usize, f32) {
    let rising = row[start + 1] > row[start];
    let mut end = start + 1;
    while end + 1 < row.len()
        && (row[end + 1] - row[end]).abs() > 2.0
        && (row[end + 1] > row[end]) == rising
    {
        end += 1;
    }
    (end, row[end] - row[start])
}

fn edge_widths(image: &image::RgbaImage) -> Vec<f32> {
    let mut widths = Vec::new();
    for y in 0..image.height() {
        let row: Vec<f32> = (0..image.width()).map(|x| luma(image, x, y)).collect();
        let mut x = 0;
        while x + 1 < row.len() {
            if (row[x + 1] - row[x]).abs() <= 2.0 {
                x += 1;
                continue;
            }
            let (end, step) = ramp(&row, x);
            if step.abs() >= MIN_STEP {
                let (low, high) = (row[x].min(row[end]), row[x].max(row[end]));
                let margin = (high - low) * 0.1;
                let inside = row[x..=end]
                    .iter()
                    .filter(|value| **value > low + margin && **value < high - margin)
                    .count();
                widths.push(inside as f32);
            }
            x = end;
        }
    }
    widths
}

fn assert_sharp(diagram: Diagram, backend: Backend) {
    assert_sharp_at(diagram, backend, ZOOM);
}

fn assert_sharp_at(diagram: Diagram, backend: Backend, zoom: f32) {
    assert_sharp_scaled(diagram, backend, zoom, 1.0);
}

fn assert_sharp_scaled(diagram: Diagram, backend: Backend, zoom: f32, scale: f32) {
    let mut ui = Harness::new(move || zoomed(diagram, zoom))
        .pixels(VIEW.0, VIEW.1)
        .scale(scale);
    ui.settle();
    let image = capture(&mut ui, backend, scale);
    let mut widths = edge_widths(&image);
    assert!(
        widths.len() >= MIN_EDGES,
        "{diagram:?} on {backend:?}: only {} strong edges in view",
        widths.len()
    );
    widths.sort_by(f32::total_cmp);
    let median = widths[widths.len() / 2];
    assert!(
        median <= MAX_EDGE,
        "{diagram:?} on {backend:?} at zoom {zoom}, scale {scale}: edges ramp over {median} px (median of {})",
        widths.len()
    );
}

#[test]
fn a_zoomed_mermaid_diagram_stays_sharp_on_cpu() {
    assert_sharp(Diagram::Mermaid, Backend::Cpu);
}

#[test]
fn a_zoomed_graphviz_graph_stays_sharp_on_cpu() {
    assert_sharp(Diagram::Graphviz, Backend::Cpu);
}

#[test]
fn a_zoomed_formula_stays_sharp_on_cpu() {
    assert_sharp(Diagram::Math, Backend::Cpu);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_zoomed_mermaid_diagram_stays_sharp_on_gpu() {
    assert_sharp(Diagram::Mermaid, Backend::Gpu);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_zoomed_graphviz_graph_stays_sharp_on_gpu() {
    assert_sharp(Diagram::Graphviz, Backend::Gpu);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_zoomed_formula_stays_sharp_on_gpu() {
    assert_sharp(Diagram::Math, Backend::Gpu);
}

#[test]
fn a_wide_diagram_stays_sharp_past_the_bitmap_cap_on_cpu() {
    assert_sharp_at(Diagram::Chain, Backend::Cpu, DEEP_ZOOM);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_wide_diagram_stays_sharp_past_the_bitmap_cap_on_gpu() {
    assert_sharp_at(Diagram::Chain, Backend::Gpu, DEEP_ZOOM);
}

#[test]
fn a_chart_line_stays_sharp_at_2x_and_after_a_4x_zoom_on_cpu() {
    assert_sharp_scaled(Diagram::Chart, Backend::Cpu, 1.0, 2.0);
    assert_sharp_scaled(Diagram::Chart, Backend::Cpu, ZOOM, 2.0);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_chart_line_stays_sharp_at_2x_and_after_a_4x_zoom_on_gpu() {
    assert_sharp_scaled(Diagram::Chart, Backend::Gpu, 1.0, 2.0);
    assert_sharp_scaled(Diagram::Chart, Backend::Gpu, ZOOM, 2.0);
}
