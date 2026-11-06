#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small and positive"
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Tree, fontdb};
use sigil::geometry::{PhysicalSize, Rect};
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use sigil::scene::paint_backdrop;
use sigil::style::Color;
use sigil_kittest::{Harness, Query};

type Picture = image::RgbaImage;

const SIZE: (u32, u32) = (300, 300);
const WAIT: Duration = Duration::from_secs(20);
const GPU_CHANNEL_TOLERANCE: i16 = 3;
const CPU_CHANNEL_TOLERANCE: i16 = 12;
const STEM_TOLERANCE: f32 = 0.25;

const RULE: &str = r##"{"width":120,"height":120,"padding":0,"config":{"view":{"stroke":null}},"data":{"values":[{}]},"mark":{"type":"rule","strokeWidth":40,"color":"#000000","opacity":0.5},"encoding":{"y":{"datum":0.5,"type":"quantitative","scale":{"domain":[0,1]},"axis":null}}}"##;
const GLYPH: &str = r##"{"width":120,"height":120,"padding":0,"config":{"view":{"stroke":null}},"data":{"values":[{}]},"mark":{"type":"text","text":"l","fontSize":80,"fontWeight":"bold","color":"#000000"}}"##;

#[derive(Clone, Copy, Debug)]
enum Backend {
    Cpu,
    Gpu,
}

impl Backend {
    const fn channel_tolerance(self) -> i16 {
        match self {
            Self::Cpu => CPU_CHANNEL_TOLERANCE,
            Self::Gpu => GPU_CHANNEL_TOLERANCE,
        }
    }
}

fn mounted(spec: &str, scale: f32) -> Harness {
    let source = format!("```vega-lite\n{spec}\n```");
    let mut ui = Harness::new(move || markdown(signal(source.clone())))
        .pixels(SIZE.0, SIZE.1)
        .scale(scale);
    let start = Instant::now();
    ui.settle();
    while ui.query(&Query::text("Rendering chart")).is_some() {
        assert!(start.elapsed() < WAIT, "still rendering after {WAIT:?}");
        std::thread::sleep(Duration::from_millis(5));
        ui.settle();
    }
    ui
}

fn capture(ui: &mut Harness, backend: Backend, scale: f32) -> Picture {
    match backend {
        Backend::Gpu => ui.image().expect("render").expect("gpu"),
        Backend::Cpu => {
            let viewport = (SIZE.0 as f32, SIZE.1 as f32);
            let host = ui.host();
            let mut scene = host.emit(viewport);
            paint_backdrop(&mut scene, host.theme(), viewport);
            let device = PhysicalSize::new(
                (SIZE.0 as f32 * scale).round() as u32,
                (SIZE.1 as f32 * scale).round() as u32,
            );
            let target = ImageTarget {
                size: device,
                viewport,
                clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
            };
            render_to_image(ImageRenderer::Cpu(&mut CpuRenderer::new()), &scene, target)
                .expect("cpu render")
        }
    }
}

fn picture_bounds(ui: &mut Harness) -> Rect {
    ui.find_all(&Query::role(Role::Image))
        .into_iter()
        .find_map(|node| node.bounds())
        .unwrap_or_else(|| panic!("no chart image: {}", ui.text()))
}

fn surface(ui: &mut Harness) -> [u8; 3] {
    let color = ui.host().theme().color(Token::Surface);
    [color.r, color.g, color.b]
}

fn fonts() -> Arc<fontdb::Database> {
    let mut fonts = fontdb::Database::new();
    for face in [
        sigil_fonts::FIRA_SANS_REGULAR,
        sigil_fonts::FIRA_SANS_MEDIUM,
        sigil_fonts::FIRA_SANS_SEMIBOLD,
    ] {
        fonts.load_font_data(face.to_vec());
    }
    Arc::new(fonts)
}

fn resvg_raster(svg: &str, scale: f32) -> Picture {
    let options = usvg::Options {
        fontdb: fonts(),
        ..usvg::Options::default()
    };
    let tree = Tree::from_str(svg, &options).expect("svg");
    let size = tree.size();
    let (width, height) = (
        (size.width() * scale).round() as u32,
        (size.height() * scale).round() as u32,
    );
    let mut pixmap = Pixmap::new(width, height).expect("pixmap");
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Picture::from_raw(width, height, rgba).expect("image")
}

fn half_alpha_reference([r, g, b]: [u8; 3]) -> [u8; 3] {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="#{r:02x}{g:02x}{b:02x}"/><line x1="0" y1="10" x2="20" y2="10" stroke="#000000" stroke-opacity="0.5" stroke-width="10"/></svg>"##
    );
    let [r, g, b, _] = resvg_raster(&svg, 1.0).get_pixel(10, 10).0;
    [r, g, b]
}

fn stem_reference(backdrop: [u8; 3], scale: f32) -> f32 {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"><text x="60" y="90" text-anchor="middle" font-family="Fira Sans" font-size="80" font-weight="bold" fill="#000000">l</text></svg>"##;
    let picture = resvg_raster(svg, scale);
    let rect = Rect::new(sigil::geometry::Point::new(0.0, 0.0), 120.0, 120.0);
    stem_width(&picture, rect, scale, backdrop, true)
}

fn coverage(pixel: [u8; 4], backdrop: [u8; 3], straight: bool) -> f32 {
    let alpha = if straight { f32::from(pixel[3]) / 255.0 } else { 1.0 };
    let shown = f32::from(pixel[1]).mul_add(alpha, f32::from(backdrop[1]) * (1.0 - alpha));
    let paper = f32::from(backdrop[1]).max(1.0);
    ((paper - shown) / paper).clamp(0.0, 1.0)
}

fn stem_width(picture: &Picture, rect: Rect, scale: f32, backdrop: [u8; 3], straight: bool) -> f32 {
    let (left, top) = ((rect.origin.x * scale) as u32, (rect.origin.y * scale) as u32);
    let right = (((rect.origin.x + rect.width) * scale) as u32).min(picture.width());
    let bottom = (((rect.origin.y + rect.height) * scale) as u32).min(picture.height());
    let inked = |y: u32| (left..right).any(|x| coverage(picture.get_pixel(x, y).0, backdrop, straight) > 0.5);
    let rows: Vec<u32> = (top..bottom).filter(|y| inked(*y)).collect();
    let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
        panic!("no glyph inked in {rect:?}");
    };
    let middle = u32::midpoint(*first, *last);
    (left..right)
        .map(|x| coverage(picture.get_pixel(x, middle).0, backdrop, straight))
        .sum()
}

fn rule_probe(backend: Backend, scale: f32) -> ([u8; 3], [u8; 3]) {
    let mut ui = mounted(RULE, scale);
    let rect = picture_bounds(&mut ui);
    let picture = capture(&mut ui, backend, scale);
    let x = ((rect.origin.x + rect.width / 2.0) * scale) as u32;
    let y = ((rect.origin.y + rect.height / 2.0) * scale) as u32;
    let [r, g, b, _] = picture.get_pixel(x, y).0;
    ([r, g, b], half_alpha_reference(surface(&mut ui)))
}

fn assert_half_alpha(backend: Backend, scale: f32) {
    let (got, want) = rule_probe(backend, scale);
    let off = got
        .iter()
        .zip(want)
        .map(|(got, want)| (i16::from(*got) - i16::from(want)).abs())
        .max()
        .unwrap_or(0);
    assert!(
        off <= backend.channel_tolerance(),
        "{backend:?} at scale {scale}: half-alpha stroke reads {got:?}, resvg composites {want:?}"
    );
}

fn assert_bold_stem(backend: Backend, scale: f32) {
    let mut ui = mounted(GLYPH, scale);
    let rect = picture_bounds(&mut ui);
    let backdrop = surface(&mut ui);
    let picture = capture(&mut ui, backend, scale);
    let got = stem_width(&picture, rect, scale, backdrop, false);
    let want = stem_reference(backdrop, scale);
    assert!(
        (got - want).abs() <= STEM_TOLERANCE * scale,
        "{backend:?} at scale {scale}: bold stem is {got:.2} px, resvg draws {want:.2} px"
    );
}

#[test]
fn a_half_alpha_stroke_composites_like_resvg_on_cpu() {
    assert_half_alpha(Backend::Cpu, 1.0);
    assert_half_alpha(Backend::Cpu, 2.0);
}

#[test]
fn a_bold_stem_keeps_its_width_on_cpu() {
    assert_bold_stem(Backend::Cpu, 1.0);
    assert_bold_stem(Backend::Cpu, 2.0);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_half_alpha_stroke_composites_like_resvg_on_gpu() {
    assert_half_alpha(Backend::Gpu, 1.0);
    assert_half_alpha(Backend::Gpu, 2.0);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_bold_stem_keeps_its_width_on_gpu() {
    assert_bold_stem(Backend::Gpu, 1.0);
    assert_bold_stem(Backend::Gpu, 2.0);
}
