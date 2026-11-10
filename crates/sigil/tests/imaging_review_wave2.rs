#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates and channels are small"
)]
#![allow(clippy::panic, reason = "a test fails loudly on a missing value")]

use std::sync::Arc;

use resvg::usvg::{self, Tree};
use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{BlendMode, CommandScene, DrawRect, SceneBuilder};
use sigil::style::Rgba;
use sigil::widgets::{SvgMode, svg_scene};
use sigil_kittest::Harness;
use sigil_kittest::gpu::headless_renderer;
use sigil_kittest::parity::{Frame, cpu_rgba, gpu_rgba};

const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

fn cpu_frame(scene: &CommandScene, side: (u32, u32)) -> image::RgbaImage {
    let frame = Frame {
        scene,
        viewport: (side.0 as f32, side.1 as f32),
        size: PhysicalSize::new(side.0, side.1),
        clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
    };
    let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    image::RgbaImage::from_raw(side.0, side.1, rgba).expect("cpu frame")
}

fn gpu_frame(scene: &CommandScene, side: (u32, u32)) -> Option<image::RgbaImage> {
    let mut renderer = headless_renderer().expect("adapter")?;
    let frame = Frame {
        scene,
        viewport: (side.0 as f32, side.1 as f32),
        size: PhysicalSize::new(side.0, side.1),
        clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
    };
    let rgba = gpu_rgba(&mut renderer, &frame).expect("gpu render");
    image::RgbaImage::from_raw(side.0, side.1, rgba)
}

fn harness_frame(ui: &mut Harness) -> image::RgbaImage {
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no GPU adapter"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

fn tree(svg: &str) -> Arc<Tree> {
    Arc::new(Tree::from_str(svg, &usvg::Options::default()).expect("svg"))
}

fn on_surface(surface: Color, svg: &str, mode: SvgMode) -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_rect(DrawRect::new(
        Rect::new(Point::new(0.0, 0.0), 40.0, 40.0),
        surface,
    ));
    let mut scene = builder.finish();
    let (picture, _) = svg_scene(tree(svg), 7, (40.0, 40.0), surface, mode);
    scene.commands.extend(picture.commands);
    scene
}

fn svg_rect(fill: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40"><rect x="5" y="5" width="30" height="30" fill="{fill}"/></svg>"#
    )
}

fn assert_ink(surface: Color, fill: &str, want: [u8; 3]) {
    let svg = svg_rect(fill);
    let mut failures = Vec::new();
    for mode in [SvgMode::Raster, SvgMode::Vector] {
        let scene = on_surface(surface, &svg, mode);
        let mut frames = vec![("cpu", cpu_frame(&scene, (40, 40)))];
        if let Some(gpu) = gpu_frame(&scene, (40, 40)) {
            frames.push(("gpu", gpu));
        }
        for (name, frame) in frames {
            let found = frame.get_pixel(20, 20).0;
            if found[..3].iter().zip(want).any(|(a, b)| a.abs_diff(b) > 4) {
                failures.push(format!("{mode:?} {name}: centre {found:?}, want {want:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "opaque {fill} rect on surface {surface:?}: {failures:#?}"
    );
}

#[test]
fn an_opaque_white_svg_rect_stays_white_on_a_light_gray_surface() {
    assert_ink(
        Color {
            r: 200,
            g: 200,
            b: 200,
            a: 255,
        },
        "#ffffff",
        [255, 255, 255],
    );
}

#[test]
fn an_opaque_black_svg_rect_stays_black_on_a_dark_surface() {
    assert_ink(
        Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        },
        "#000000",
        [0, 0, 0],
    );
}

fn halves() -> NodeBuilder {
    let half = |color: Color| {
        container()
            .layout(LayoutStyle::block().fixed_size(40.0, 40.0))
            .background(color)
    };
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(120.0, 80.0)
                .padding(Space::raw(20.0, "test fixture")),
        )
        .background(WHITE)
        .child(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(80.0, 40.0))
                .blur(4.0)
                .child(half(BLACK))
                .child(half(WHITE)),
        )
}

#[test]
fn blur_mixes_a_black_white_edge_in_srgb_as_filter_effects_requires() {
    let mut pixels = Harness::new(halves).pixels(120, 80);
    let mut surface = Harness::new(halves).gpu(120, 80, 1.0);
    let scene = surface.host().emit((120.0, 80.0));
    let frames = [
        ("pixels", harness_frame(&mut pixels)),
        ("gpu", harness_frame(&mut surface)),
        ("cpu", cpu_frame(&scene, (120, 80))),
    ];
    let mut failures = Vec::new();
    for (name, frame) in &frames {
        let left = frame.get_pixel(59, 40).0[0];
        let right = frame.get_pixel(60, 40).0[0];
        let edge = u16::midpoint(u16::from(left), u16::from(right));
        println!("{name}: edge pixels {left} {right}");
        if edge.abs_diff(128) > 16 {
            failures.push(format!(
                "{name}: edge mean {edge}, sRGB blur gives ~128, linear blur ~188"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const BACKDROP: Color = Color {
    r: 200,
    g: 100,
    b: 50,
    a: 255,
};
const SOURCE: Color = Color {
    r: 50,
    g: 150,
    b: 250,
    a: 255,
};
const MULTIPLIED: [u8; 3] = [39, 59, 49];

fn turned_multiply() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(100.0, 100.0)
                .padding(Space::raw(30.0, "test fixture")),
        )
        .background(BACKDROP)
        .perspective(400.0)
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(40.0, 40.0))
                .background(SOURCE)
                .rotate_x(5.0)
                .blend(BlendMode::Multiply),
        )
}

#[test]
fn mix_blend_on_a_3d_turned_node_still_multiplies() {
    let mut pixels = Harness::new(turned_multiply).pixels(100, 100);
    let mut surface = Harness::new(turned_multiply).gpu(100, 100, 1.0);
    let scene = surface.host().emit((100.0, 100.0));
    let frames = [
        ("pixels", harness_frame(&mut pixels)),
        ("gpu", harness_frame(&mut surface)),
        ("cpu", cpu_frame(&scene, (100, 100))),
    ];
    let mut failures = Vec::new();
    for (name, frame) in &frames {
        let found = frame.get_pixel(50, 50).0;
        if found[..3]
            .iter()
            .zip(MULTIPLIED)
            .any(|(a, b)| a.abs_diff(b) > 4)
        {
            failures.push(format!(
                "{name}: centre {found:?}, multiply gives {MULTIPLIED:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const NESTED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400">
<defs>
<pattern id="inner" width="0.5" height="0.5" patternUnits="userSpaceOnUse">
<rect width="0.25" height="0.25" fill="black"/>
</pattern>
<pattern id="outer" width="32" height="32" patternUnits="userSpaceOnUse">
<rect width="32" height="32" fill="url(#inner)"/>
</pattern>
</defs>
<rect width="384" height="384" fill="url(#outer)"/>
</svg>"#;

#[test]
fn nested_patterns_lower_to_a_bounded_number_of_commands() {
    let started = std::time::Instant::now();
    let (scene, fallbacks) = svg_scene(tree(NESTED), 9, (400.0, 400.0), WHITE, SvgMode::Vector);
    let count = scene.commands.len();
    println!(
        "{count} commands, fallbacks {fallbacks:?}, {:?}",
        started.elapsed()
    );
    assert!(
        count <= 100_000,
        "a 600-byte SVG lowered to {count} scene commands in {:?}",
        started.elapsed()
    );
}

fn red_pixels(frame: &image::RgbaImage) -> usize {
    frame
        .pixels()
        .filter(|p| p.0[0] >= 200 && p.0[1] <= 80 && p.0[2] <= 80)
        .count()
}

fn shadowed_text(on_parent: bool) -> impl Fn() -> NodeBuilder {
    move || {
        let shadow = sigil::scene::Shadow::new([3.0, 3.0], 0.0, 0.0, [1.0, 0.0, 0.0, 1.0]);
        let label = text("Shadow").font_size(32.0, "test fixture");
        let label = if on_parent {
            label
        } else {
            label.text_shadow(shadow)
        };
        let parent = container()
            .layout(LayoutStyle::block().fixed_size(200.0, 60.0))
            .background(WHITE);
        let parent = if on_parent {
            parent.text_shadow(shadow)
        } else {
            parent
        };
        parent.child(label)
    }
}

#[test]
fn text_shadow_inherits_to_child_text_like_css() {
    let mut own = Harness::new(shadowed_text(false)).pixels(200, 60);
    let mut inherited = Harness::new(shadowed_text(true)).pixels(200, 60);
    let own = red_pixels(&harness_frame(&mut own));
    let inherited = red_pixels(&harness_frame(&mut inherited));
    println!("red shadow pixels: on the text {own}, on its parent {inherited}");
    assert!(own > 50, "the probe sees a text's own shadow");
    assert!(
        inherited * 2 >= own,
        "text-shadow is inherited (CSS Text Decoration 3): parent shadow drew {inherited} red pixels, own drew {own}"
    );
}
