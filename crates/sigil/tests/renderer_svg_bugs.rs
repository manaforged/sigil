#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates and channels are small"
)]

use std::sync::Arc;

use resvg::usvg::{self, Tree};
use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{CommandScene, DrawRect, SceneBuilder};
use sigil::style::Color;
use sigil::widgets::{SvgMode, svg_scene};
use sigil_kittest::gpu::headless_renderer;
use sigil_kittest::parity::{Frame, cpu_rgba, gpu_rgba};

#[path = "renderer_svg_bugs/blur.rs"]
mod blur;
#[path = "renderer_svg_bugs/images.rs"]
mod images;
#[path = "renderer_svg_bugs/nested.rs"]
mod nested;

const SIDE: u32 = 40;
const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
const TOLERANCE: u8 = 4;

const fn frame(scene: &CommandScene, side: u32) -> Frame<'_> {
    Frame {
        scene,
        viewport: (side as f32, side as f32),
        size: PhysicalSize::new(side, side),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    }
}

pub(crate) fn frames(scene: &CommandScene, side: u32) -> Vec<(&'static str, Vec<u8>)> {
    let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame(scene, side)).expect("cpu render");
    let mut out = vec![("cpu", cpu)];
    if let Some(mut renderer) = headless_renderer().expect("adapter") {
        out.push((
            "gpu",
            gpu_rgba(&mut renderer, &frame(scene, side)).expect("gpu render"),
        ));
    }
    out
}

pub(crate) fn picture(svg: &str, side: u32) -> CommandScene {
    let tree = Arc::new(Tree::from_str(svg, &usvg::Options::default()).expect("svg"));
    let mut builder = SceneBuilder::new();
    let side = side as f32;
    builder.draw_rect(DrawRect::new(
        Rect::new(Point::new(0.0, 0.0), side, side),
        WHITE,
    ));
    let mut scene = builder.finish();
    let (lowered, fallbacks) = svg_scene(tree, 11, (side, side), WHITE, SvgMode::Vector);
    assert!(
        fallbacks.is_empty(),
        "lowered with bitmap fallbacks: {fallbacks:?}"
    );
    scene.commands.extend(lowered.commands);
    scene
}

pub(crate) fn svg(body: &str, side: u32) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}">{body}</svg>"#
    )
}

const fn pixel(rgba: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
    let at = ((y * SIDE + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

type Probe = ((u32, u32), [u8; 3]);

#[track_caller]
fn assert_pixels(body: &str, probes: &[Probe]) {
    let scene = picture(&svg(body, SIDE), SIDE);
    let mut failures = Vec::new();
    for (name, rgba) in frames(&scene, SIDE) {
        for &(at, want) in probes {
            let found = pixel(&rgba, at);
            if found
                .iter()
                .zip(want)
                .any(|(a, b)| a.abs_diff(b) > TOLERANCE)
            {
                failures.push(format!("{name} at {at:?}: {found:?}, want {want:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{body}\n{failures:#?}");
}

#[test]
fn a_translucent_svg_fill_mixes_with_white_in_srgb() {
    assert_pixels(
        r##"<rect x="5" y="5" width="30" height="30" fill="#66208a" fill-opacity="0.4"/>"##,
        &[((20, 20), [194, 166, 208])],
    );
}

#[test]
fn a_faded_multiply_group_mixes_with_its_backdrop_in_srgb() {
    assert_pixels(
        r##"<rect width="40" height="40" fill="#c8c864"/><g opacity="0.6" style="mix-blend-mode:multiply"><rect x="5" y="5" width="30" height="30" fill="#6496fa"/></g>"##,
        &[((20, 20), [127, 151, 99]), ((2, 2), [200, 200, 100])],
    );
}

#[test]
fn a_multiply_group_over_a_translucent_backdrop_follows_the_css_formula() {
    assert_pixels(
        r##"<rect width="40" height="40" fill="#c8c864" fill-opacity="0.5"/><g style="mix-blend-mode:multiply"><rect x="5" y="5" width="30" height="30" fill="#6496fa"/></g>"##,
        &[((20, 20), [89, 134, 174]), ((2, 2), [228, 228, 178])],
    );
}

#[test]
fn a_clip_path_on_a_child_of_a_faded_group_clips_it() {
    assert_pixels(
        r##"<clipPath id="c"><circle cx="20" cy="20" r="10"/></clipPath><g opacity="0.5"><rect width="40" height="40" fill="#0000ff" clip-path="url(#c)"/></g>"##,
        &[
            ((20, 20), [128, 128, 255]),
            ((4, 4), [255, 255, 255]),
            ((36, 20), [255, 255, 255]),
        ],
    );
}

#[test]
fn a_clip_path_on_a_child_of_a_masked_group_clips_it() {
    assert_pixels(
        r##"<clipPath id="c"><circle cx="20" cy="20" r="10"/></clipPath><mask id="m"><rect width="40" height="40" fill="#ffffff"/></mask><g mask="url(#m)"><rect width="40" height="40" fill="#0000ff" clip-path="url(#c)"/></g>"##,
        &[
            ((20, 20), [0, 0, 255]),
            ((4, 4), [255, 255, 255]),
            ((36, 20), [255, 255, 255]),
        ],
    );
}
