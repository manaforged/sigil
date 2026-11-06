use std::sync::Arc;

use resvg::usvg::{self, Tree};
use sigil::geometry::{Point, Rect};
use sigil::scene::{CommandScene, DrawRect, SceneBuilder};
use sigil::style::Rgba8;
use sigil::widgets::SvgMode;

use super::{
    Backend, CHANNEL_TOLERANCE, MAX_OFF_FRACTION, Renderers, WHITE, off_fraction, over,
    resvg_picture, scene,
};

const SURFACES: [Rgba8; 3] = [
    WHITE,
    Rgba8 {
        r: 200,
        g: 200,
        b: 200,
        a: 255,
    },
    Rgba8 {
        r: 40,
        g: 44,
        b: 52,
        a: 255,
    },
];

const PROBES: [(&str, &str); 10] = [
    (
        "reflected rotated linear gradient",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><linearGradient id="g" x1="0.3" x2="0.6" spreadMethod="reflect" gradientTransform="rotate(30 0.5 0.5)"><stop offset="0" stop-color="#1060d0"/><stop offset="1" stop-color="#f0a020"/></linearGradient><rect width="100" height="100" fill="url(#g)"/></svg>"##,
    ),
    (
        "radial gradient with stop opacity",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><radialGradient id="g"><stop offset="0" stop-color="#d02020"/><stop offset="1" stop-color="#2020d0" stop-opacity="0"/></radialGradient><rect width="100" height="100" fill="url(#g)"/></svg>"##,
    ),
    (
        "clip path of two shapes",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><clipPath id="c"><circle cx="35" cy="50" r="30"/><rect x="50" y="10" width="40" height="40"/></clipPath><rect width="100" height="100" fill="#208040" clip-path="url(#c)"/></svg>"##,
    ),
    (
        "luminance mask",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><linearGradient id="g"><stop offset="0" stop-color="white"/><stop offset="1" stop-color="black"/></linearGradient><mask id="m"><rect width="100" height="100" fill="url(#g)"/></mask><rect width="100" height="100" fill="#3050c0" mask="url(#m)"/></svg>"##,
    ),
    (
        "gaussian blur",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><filter id="f"><feGaussianBlur stdDeviation="4"/></filter><rect x="25" y="25" width="50" height="50" fill="#c03030" filter="url(#f)"/></svg>"##,
    ),
    (
        "pattern fill",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><pattern id="p" width="20" height="20" patternUnits="userSpaceOnUse"><rect width="10" height="10" fill="#204080"/><circle cx="15" cy="15" r="4" fill="#e08020"/></pattern><circle cx="50" cy="50" r="45" fill="url(#p)"/></svg>"##,
    ),
    (
        "dashed stroke with offset and miter limit",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path d="M10 80 L50 20 L90 80" fill="none" stroke="#202020" stroke-width="8" stroke-dasharray="14 6" stroke-dashoffset="5" stroke-miterlimit="10"/></svg>"##,
    ),
    (
        "group opacity and multiply blend",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="10" y="10" width="60" height="60" fill="#30a0e0"/><g opacity="0.6" style="mix-blend-mode:multiply"><circle cx="60" cy="60" r="30" fill="#e04040"/><circle cx="45" cy="70" r="20" fill="#40c040"/></g></svg>"##,
    ),
    (
        "translucent fill drawn after a faded group",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#b0c4de"/><g opacity="0.5"><rect x="4" y="4" width="8" height="8" fill="#30a0e0"/><rect x="8" y="8" width="8" height="8" fill="#e04040"/></g><rect x="10" y="20" width="80" height="70" fill="#000000" fill-opacity="0.7"/></svg>"##,
    ),
    (
        "translucent fill under a clip path",
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><clipPath id="c"><circle cx="50" cy="50" r="45"/></clipPath><rect width="100" height="100" fill="#b0c4de"/><g clip-path="url(#c)"><rect width="100" height="100" fill="#000000" fill-opacity="0.7"/></g></svg>"##,
    ),
];

const GPU_PROBES: [(&str, &str); 1] = [(
    "element opacity over an opaque mark",
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#b0c4de"/><rect x="10" y="10" width="80" height="80" fill="#000000" opacity="0.7"/></svg>"##,
)];

fn on_surface(surface: Rgba8, picture: CommandScene) -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_rect(DrawRect::new(
        Rect::new(Point::new(0.0, 0.0), 100.0, 100.0),
        surface,
    ));
    let mut scene = builder.finish();
    scene.commands.extend(picture.commands);
    scene
}

fn probe(backend: Backend) {
    let mut renderers = Renderers::new(backend);
    let mut failed = Vec::new();
    let extra = match backend {
        Backend::Gpu => GPU_PROBES.as_slice(),
        Backend::Cpu => &[],
    };
    for &(name, source) in PROBES.iter().chain(extra) {
        let tree = Arc::new(Tree::from_str(source, &usvg::Options::default()).expect("svg"));
        let reference = resvg_picture(&tree, (100, 100));
        for surface in SURFACES {
            let want = over(&reference, surface);
            let (lowered, fallbacks) = scene(&tree, name, (100, 100), SvgMode::Vector);
            let got = renderers
                .render(&on_surface(surface, lowered), (100, 100))
                .expect("render");
            let off = off_fraction(&got, &want, CHANNEL_TOLERANCE);
            if !fallbacks.is_empty() || off > MAX_OFF_FRACTION {
                failed.push(format!(
                    "{name} over {surface:?}: {:.1}% off, fallbacks {fallbacks:?}",
                    off * 100.0
                ));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "{backend:?} probes differ from resvg: {failed:?}"
    );
}

#[test]
fn each_probe_lowers_to_scene_commands_that_match_resvg_on_cpu() {
    probe(Backend::Cpu);
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn each_probe_lowers_to_scene_commands_that_match_resvg_on_gpu() {
    probe(Backend::Gpu);
}
