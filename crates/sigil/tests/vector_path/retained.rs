use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::renderer::{CpuRenderer, WgpuRenderer};
use sigil::scene::{CommandScene, SceneBuilder};
use sigil::style::{Color, Theme, Token};
use sigil::widgets::{PathNode, path};
use sigil_kittest::gpu::{headless_renderer, pixel_rgba};
use sigil_kittest::parity::{Frame, cpu_rgba, gpu_rgba};

const SIDE: f32 = 60.0;
const PIXELS: u32 = 120;
const CLEAR: Color = Color::rgba(0.5, 0.5, 0.5, 1.0);

fn scene_of(node: PathNode, theme: &Theme) -> CommandScene {
    let mut scene = SceneBuilder::new();
    node.draw(
        &mut scene,
        Rect::new(Point::new(0.0, 0.0), SIDE, SIDE),
        theme,
    );
    scene.finish()
}

fn wedge(tip: (f32, f32)) -> PathNode {
    path(60, 60)
        .move_to(0.0, 0.0)
        .line_to(60.0, 0.0)
        .line_to(60.0, 60.0)
        .line_to(0.0, 60.0)
        .line_to(tip.0, tip.1)
        .close()
        .fill(Token::OnSurface)
}

fn frame(scene: &CommandScene) -> Frame<'_> {
    Frame {
        scene,
        viewport: (SIDE, SIDE),
        size: PhysicalSize::new(PIXELS, PIXELS),
        clear: CLEAR,
    }
}

fn gpu(renderer: &mut WgpuRenderer, scene: &CommandScene) -> Vec<u8> {
    gpu_rgba(renderer, &frame(scene)).expect("gpu render")
}

fn probe(rgba: &[u8], (x, y): (f32, f32)) -> [u8; 4] {
    pixel_rgba(rgba, PIXELS, (x * 2.0) as u32, (y * 2.0) as u32)
}

fn near(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(b).take(3).all(|(x, y)| x.abs_diff(y) <= 8)
}

#[test]
fn a_path_whose_points_change_between_frames_redraws() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        panic!("no GPU adapter");
    };
    let theme = Theme::light();
    let shallow = scene_of(wedge((10.0, 30.0)), &theme);
    let deep = scene_of(wedge((50.0, 30.0)), &theme);
    let first = gpu(&mut renderer, &shallow);
    let again = gpu(&mut renderer, &shallow);
    let moved = gpu(&mut renderer, &deep);
    let back = gpu(&mut renderer, &shallow);
    assert_eq!(
        first, again,
        "an unchanged path draws the same pixels on its second frame"
    );
    let ink = probe(&first, (55.0, 30.0));
    assert!(
        near(probe(&moved, (55.0, 30.0)), ink),
        "both wedges cover the right edge"
    );
    assert!(
        near(probe(&first, (30.0, 30.0)), ink),
        "the shallow notch leaves the centre filled"
    );
    assert!(
        !near(probe(&moved, (30.0, 30.0)), ink),
        "the deep notch opens the centre"
    );
    assert_eq!(
        first, back,
        "returning to the first points draws the first pixels"
    );
}

#[test]
fn a_theme_change_recolors_a_themed_path() {
    let Some(mut renderer) = headless_renderer().expect("sigil-kittest") else {
        panic!("no GPU adapter");
    };
    let node = || wedge((15.0, 30.0));
    let inside = (45.0, 45.0);
    let mut seen = Vec::new();
    for theme in [Theme::light(), Theme::dark(), Theme::light()] {
        let scene = scene_of(node(), &theme);
        let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame(&scene)).expect("cpu render");
        let drawn = probe(&gpu(&mut renderer, &scene), inside);
        assert!(
            near(drawn, probe(&cpu, inside)),
            "the gpu fill {drawn:?} matches the cpu fill {:?}",
            probe(&cpu, inside)
        );
        seen.push(drawn);
    }
    assert!(
        !near(seen[0], seen[1]),
        "dark recolors the light fill: {seen:?}"
    );
    assert_eq!(seen[0], seen[2], "light again restores the light fill");
}
