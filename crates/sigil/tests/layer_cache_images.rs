#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]
#![allow(clippy::cast_precision_loss, reason = "pixel sides are small")]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::{ImageSource, ObjectFit};
use sigil::renderer::{CpuRenderer, WgpuRenderer};
use sigil::scene::{
    BlendMode, CommandScene, CornerRadii, DrawImage, DrawRect, Filter, LayerEffect, LayerScope,
    SceneBuilder,
};
use sigil::style::{Color, Rgba};
use sigil_kittest::gpu::headless_renderer;
use sigil_kittest::parity::{Frame, cpu_rgba, gpu_rgba};

const SIDE: u32 = 64;
const SCALE: u32 = 2;
const FILLER: usize = 12;
const RED: Color = Color {
    r: 230,
    g: 20,
    b: 20,
    a: 255,
};
const BLUE: Color = Color {
    r: 20,
    g: 20,
    b: 230,
    a: 255,
};
const GRAY: Color = Color {
    r: 90,
    g: 90,
    b: 90,
    a: 255,
};

fn solid(color: Color) -> ImageSource {
    let rgba = [color.r, color.g, color.b, color.a].repeat((SIDE * SIDE) as usize);
    ImageSource::from_rgba8(SIDE, SIDE, rgba).expect("image")
}

fn scene(color: Color) -> CommandScene {
    let area = Rect::new(Point::new(0.0, 0.0), SIDE as f32, SIDE as f32);
    let scope = LayerScope {
        clip: area,
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Filter(vec![Filter::Brightness(1.0)].into()),
        opacity: 1.0,
        blend: BlendMode::default(),
    };
    let mut scene = SceneBuilder::new();
    scene.with_composite_layer(scope, |scene| {
        for at in 0..FILLER {
            let corner = Rect::new(Point::new(at as f32, 0.0), 1.0, 1.0);
            scene.draw_rect(DrawRect::new(corner, GRAY));
        }
        scene.draw_image(DrawImage {
            node: None,
            source: solid(color),
            fit: ObjectFit::Fill,
            destination: Rect::new(Point::new(8.0, 8.0), 48.0, 48.0),
            opacity: 10_000,
            crop: None,
            raster: None,
        });
    });
    scene.finish()
}

const fn frame(scene: &CommandScene) -> Frame<'_> {
    Frame {
        scene,
        viewport: (SIDE as f32, SIDE as f32),
        size: PhysicalSize::new(SIDE * SCALE, SIDE * SCALE),
        clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
    }
}

const fn center(rgba: &[u8]) -> [u8; 3] {
    let side = (SIDE * SCALE) as usize;
    let at = (side / 2 * side + side / 2) * 4;
    [rgba[at], rgba[at + 1], rgba[at + 2]]
}

fn assert_follows(mut render: impl FnMut(&CommandScene) -> Vec<u8>, backend: &str) {
    let mut seen = Vec::new();
    for (color, name) in [(RED, "red"), (RED, "red"), (BLUE, "blue"), (RED, "red")] {
        let got = center(&render(&scene(color)));
        let near = got
            .iter()
            .zip([color.r, color.g, color.b])
            .all(|(got, want)| got.abs_diff(want) <= 6);
        seen.push(format!("{name}->{got:?}"));
        assert!(
            near,
            "{backend}: the cached layer kept old image pixels: {seen:?}"
        );
    }
}

#[test]
fn a_cached_cpu_layer_shows_the_new_pixels_of_a_changed_image() {
    let mut renderer = CpuRenderer::new();
    assert_follows(
        |scene| cpu_rgba(&mut renderer, &frame(scene)).expect("cpu"),
        "cpu",
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn a_kept_gpu_layer_shows_the_new_pixels_of_a_changed_image() {
    let mut renderer: WgpuRenderer = headless_renderer().expect("kittest").expect("adapter");
    assert_follows(
        |scene| gpu_rgba(&mut renderer, &frame(scene)).expect("gpu"),
        "gpu",
    );
}
