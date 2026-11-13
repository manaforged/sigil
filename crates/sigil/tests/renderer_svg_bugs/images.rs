use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::ImageSource;
use sigil::scene::{CommandScene, DrawImage, SceneBuilder};
use sigil::style::Rgba;
use sigil_kittest::gpu::headless_renderer;
use sigil_kittest::parity::{Frame, gpu_rgba};

const SIDE: u32 = 300;
const FRAMES: u32 = 400;
const VIEW: u32 = 16;

fn solid(index: u32) -> ImageSource {
    let color = [
        (index % 200) as u8 + 40,
        (index / 200) as u8 * 100 + 20,
        200,
        255,
    ];
    ImageSource::from_rgba8(SIDE, SIDE, color.repeat((SIDE * SIDE) as usize)).expect("image")
}

fn scene(source: ImageSource) -> CommandScene {
    let mut scene = SceneBuilder::new();
    scene.draw_image(DrawImage::new(
        source,
        Rect::new(Point::new(0.0, 0.0), VIEW as f32, VIEW as f32),
    ));
    scene.finish()
}

#[test]
fn one_renderer_draws_every_new_image_across_many_frames() {
    let Some(mut renderer) = headless_renderer().expect("adapter") else {
        return;
    };
    let mut missing = Vec::new();
    for index in 0..FRAMES {
        let source = solid(index);
        let want = source.rgba()[..3].to_vec();
        let scene = scene(source);
        let frame = Frame::new(
            &scene,
            (VIEW as f32, VIEW as f32),
            PhysicalSize::new(VIEW, VIEW),
            Rgba::new(0.0, 0.0, 0.0, 1.0),
        );
        let rgba = gpu_rgba(&mut renderer, &frame).expect("gpu render");
        let at = ((VIEW / 2 * VIEW + VIEW / 2) * 4) as usize;
        let found = &rgba[at..at + 3];
        if found.iter().zip(&want).any(|(a, b)| a.abs_diff(*b) > 2) {
            missing.push((index, found.to_vec(), want));
        }
    }
    assert!(
        missing.is_empty(),
        "{} of {FRAMES} frames drew the wrong image: {:?}",
        missing.len(),
        missing.first()
    );
}
