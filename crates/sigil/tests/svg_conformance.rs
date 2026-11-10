#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "pixel sizes and counts are small"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]
#![allow(clippy::print_stdout, reason = "the check prints its pass count")]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Tree, fontdb};
use sigil::geometry::PhysicalSize;
use sigil::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use sigil::renderer::{CpuRenderer, WgpuRenderer};
use sigil::scene::CommandScene;
use sigil::style::{Color, Rgba8};
use sigil::widgets::{SvgMode, svg_scene};
use sigil_kittest::gpu::headless_renderer;
use sigil_kittest::parity::{Frame, gpu_rgba};

#[path = "svg_conformance/probes.rs"]
mod probes;
#[path = "svg_conformance/report.rs"]
mod report;

type Picture = image::RgbaImage;

const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
const CHANNEL_TOLERANCE: u8 = 32;
const MAX_OFF_FRACTION: f32 = 0.02;
const SUITE_TAG: &str = "v0.48.1";

struct Case {
    category: String,
    name: String,
    svg: PathBuf,
    reference: PathBuf,
}

#[derive(Default)]
struct Tally {
    tests: usize,
    raster: usize,
    vector: usize,
    gpu_tests: usize,
    gpu_vector: usize,
    fallbacks: BTreeMap<String, usize>,
}

fn key(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
    })
}

fn over_white(picture: &Picture) -> Picture {
    over(picture, WHITE)
}

fn over(picture: &Picture, surface: Rgba8) -> Picture {
    let mut out = picture.clone();
    for pixel in out.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        let alpha = f32::from(a) / 255.0;
        let mix = |c: u8, under: u8| {
            f32::from(c)
                .mul_add(alpha, f32::from(under) * (1.0 - alpha))
                .round() as u8
        };
        pixel.0 = [mix(r, surface.r), mix(g, surface.g), mix(b, surface.b), 255];
    }
    out
}

#[derive(Clone, Copy, Debug)]
enum Backend {
    Cpu,
    Gpu,
}

struct Renderers {
    cpu: CpuRenderer,
    gpu: Option<WgpuRenderer>,
    backend: Backend,
}

impl Renderers {
    fn new(backend: Backend) -> Self {
        let gpu = match backend {
            Backend::Cpu => None,
            Backend::Gpu => Some(headless_renderer().expect("adapter").expect("gpu")),
        };
        Self {
            cpu: CpuRenderer::new(),
            gpu,
            backend,
        }
    }

    fn render(&mut self, scene: &CommandScene, size: (u32, u32)) -> Option<Picture> {
        match (self.backend, self.gpu.as_mut()) {
            (Backend::Gpu, Some(device)) => gpu(device, scene, size),
            _ => cpu(&mut self.cpu, scene, size),
        }
    }
}

fn off_fraction(got: &Picture, want: &Picture, tolerance: u8) -> f32 {
    if got.dimensions() != want.dimensions() {
        return 1.0;
    }
    let off = got
        .pixels()
        .zip(want.pixels())
        .filter(|(a, b)| {
            a.0.iter()
                .zip(b.0)
                .take(3)
                .any(|(x, y)| x.abs_diff(y) > tolerance)
        })
        .count();
    off as f32 / (got.width() * got.height()) as f32
}

fn passes(got: &Picture, want: &Picture) -> bool {
    off_fraction(got, want, CHANNEL_TOLERANCE) <= MAX_OFF_FRACTION
}

const fn target(width: u32, height: u32) -> ImageTarget {
    ImageTarget {
        size: PhysicalSize::new(width, height),
        viewport: (width as f32, height as f32),
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
    }
}

fn cpu(
    renderer: &mut CpuRenderer,
    scene: &CommandScene,
    (width, height): (u32, u32),
) -> Option<Picture> {
    render_to_image(ImageRenderer::Cpu(renderer), scene, target(width, height)).ok()
}

fn gpu(
    renderer: &mut WgpuRenderer,
    scene: &CommandScene,
    (width, height): (u32, u32),
) -> Option<Picture> {
    let frame = Frame {
        scene,
        viewport: (width as f32, height as f32),
        size: PhysicalSize::new(width, height),
        clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
    };
    let rgba = gpu_rgba(renderer, &frame).ok()?;
    Picture::from_raw(width, height, rgba)
}

fn scene(
    tree: &Arc<Tree>,
    name: &str,
    size: (u32, u32),
    mode: SvgMode,
) -> (CommandScene, Vec<String>) {
    svg_scene(
        Arc::clone(tree),
        key(name),
        (size.0 as f32, size.1 as f32),
        WHITE,
        mode,
    )
}

fn resvg_picture(tree: &Tree, (width, height): (u32, u32)) -> Picture {
    let mut pixmap = Pixmap::new(width, height).expect("pixmap");
    let size = tree.size();
    let fit = Transform::from_scale(width as f32 / size.width(), height as f32 / size.height());
    resvg::render(tree, fit, &mut pixmap.as_mut());
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

fn suite() -> PathBuf {
    std::env::var_os("SIGIL_RESVG_SUITE").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/resvg-src/crates/resvg/tests"),
        PathBuf::from,
    )
}

fn cases(root: &Path) -> Vec<Case> {
    let mut found = Vec::new();
    let mut stack = vec![root.join("tests")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "svg") {
                let reference = path.with_extension("png");
                let relative = path.strip_prefix(root.join("tests")).unwrap_or(&path);
                let parts: Vec<String> = relative
                    .iter()
                    .map(|part| part.to_string_lossy().into_owned())
                    .collect();
                if reference.exists() && parts.len() >= 3 {
                    found.push(Case {
                        category: format!("{}/{}", parts[0], parts[1]),
                        name: relative.with_extension("").to_string_lossy().into_owned(),
                        svg: path,
                        reference,
                    });
                }
            }
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn fonts(root: &Path) -> Arc<fontdb::Database> {
    let mut fonts = fontdb::Database::new();
    for entry in std::fs::read_dir(root.join("fonts"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext == "ttf" || ext == "otf")
            && let Ok(data) = std::fs::read(&path)
        {
            fonts.load_font_data(data);
        }
    }
    fonts.set_serif_family("Noto Serif");
    fonts.set_sans_serif_family("Noto Sans");
    fonts.set_cursive_family("Yellowtail");
    fonts.set_fantasy_family("Sedgwick Ave Display");
    fonts.set_monospace_family("Noto Mono");
    Arc::new(fonts)
}

fn load(case: &Case, fonts: &Arc<fontdb::Database>) -> Option<(Arc<Tree>, Picture)> {
    let options = usvg::Options {
        fontdb: Arc::clone(fonts),
        resources_dir: case.svg.parent().map(Path::to_path_buf),
        ..usvg::Options::default()
    };
    let data = std::fs::read(&case.svg).ok()?;
    let tree = Tree::from_data(&data, &options).ok()?;
    let reference = image::open(&case.reference).ok()?.to_rgba8();
    Some((Arc::new(tree), over_white(&reference)))
}

struct Runner {
    cpu: CpuRenderer,
    gpu: Option<WgpuRenderer>,
    failures: String,
}

impl Runner {
    fn run(&mut self, case: &Case, fonts: &Arc<fontdb::Database>, tally: &mut Tally) {
        tally.tests += 1;
        let Some((tree, want)) = catch_unwind(AssertUnwindSafe(|| load(case, fonts)))
            .ok()
            .flatten()
        else {
            let _ = writeln!(self.failures, "{} load", case.name);
            return;
        };
        let size = want.dimensions();
        let rendered = catch_unwind(AssertUnwindSafe(|| {
            let (raster, _) = scene(&tree, &case.name, size, SvgMode::Raster);
            let (vector, fallbacks) = scene(&tree, &case.name, size, SvgMode::Vector);
            let raster = cpu(&mut self.cpu, &raster, size);
            let lowered = cpu(&mut self.cpu, &vector, size);
            let sampled = self
                .gpu
                .as_mut()
                .and_then(|device| gpu(device, &vector, size));
            (raster, lowered, sampled, fallbacks)
        }));
        let Ok((raster, lowered, sampled, fallbacks)) = rendered else {
            let _ = writeln!(self.failures, "{} panic", case.name);
            return;
        };
        tally.raster += usize::from(raster.is_some_and(|got| passes(&got, &want)));
        let vector_ok = lowered.is_some_and(|got| passes(&got, &want));
        tally.vector += usize::from(vector_ok);
        if let Some(got) = sampled {
            let gpu_ok = passes(&got, &want);
            tally.gpu_tests += 1;
            tally.gpu_vector += usize::from(gpu_ok);
            if !gpu_ok {
                let _ = writeln!(self.failures, "{} gpu", case.name);
            }
        }
        for reason in &fallbacks {
            *tally.fallbacks.entry(reason.clone()).or_default() += 1;
        }
        if !vector_ok {
            let _ = writeln!(
                self.failures,
                "{} vector fallbacks={fallbacks:?}",
                case.name
            );
        }
    }
}

#[test]
#[ignore = "manual: needs the resvg test suite at SIGIL_RESVG_SUITE or target/resvg-src"]
fn the_resvg_suite_renders_through_sigil() {
    let root = suite();
    let cases = cases(&root);
    assert!(!cases.is_empty(), "no resvg tests under {}", root.display());
    let fonts = fonts(&root);
    let mut runner = Runner {
        cpu: CpuRenderer::new(),
        gpu: headless_renderer().ok().flatten(),
        failures: String::new(),
    };
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for case in &cases {
        runner.run(
            case,
            &fonts,
            tallies.entry(case.category.clone()).or_default(),
        );
    }
    let out = std::env::var_os("SIGIL_CONFORMANCE_OUT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/svg-conformance"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&out).expect("results dir");
    let summary = report::report(&tallies);
    std::fs::write(out.join("results.tsv"), &summary).expect("results");
    std::fs::write(out.join("failures.txt"), &runner.failures).expect("failures");
    println!("{summary}");
}
