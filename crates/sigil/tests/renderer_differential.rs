#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]

use sigil::renderer::{CpuRenderer, WgpuRenderer};
use sigil_kittest::parity::{
    Diff, Frame, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, cpu_rgba, diff, gpu_rgba, minimize,
};
use sigil_kittest::scene_gen::{GeneratedScene, fixture_text, stress};

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn compare(gpu: &mut WgpuRenderer, scene: &GeneratedScene) -> Result<Diff, String> {
    let commands = scene.scene();
    let frame = Frame {
        scene: &commands,
        viewport: scene.viewport,
        size: scene.physical(),
        clear: scene.clear,
    };
    let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame).map_err(|e| format!("cpu: {e:?}"))?;
    let gpu = gpu_rgba(gpu, &frame).map_err(|e| format!("gpu: {e:?}"))?;
    Ok(diff(&gpu, &cpu))
}

fn diverges(d: &Result<Diff, String>) -> bool {
    !d.as_ref()
        .is_ok_and(|d| d.mean <= MAX_MEAN_DIFF && d.close >= MIN_CLOSE_FRACTION)
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware"]
fn gpu_matches_cpu_on_generated_scenes() {
    let Some(mut gpu) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        println!("no adapter; skipping differential run");
        return;
    };
    let start = env_u64("SIGIL_DIFF_START", 0);
    let count = env_u64("SIGIL_DIFF_COUNT", 500);
    let mut failures = Vec::new();
    for seed in start..start + count {
        let scene = GeneratedScene::from_seed(seed);
        let d = compare(&mut gpu, &scene);
        if !diverges(&d) {
            continue;
        }
        let keep = minimize(scene.commands.len(), |keep| {
            diverges(&compare(&mut gpu, &scene.keep(keep)))
        });
        let small = scene.keep(&keep);
        println!(
            "seed {seed}: {d:?}\n{}viewport {:?} scale {}\n{:#?}",
            fixture_text(seed, &keep),
            small.viewport,
            small.scale,
            small.commands
        );
        failures.push(seed);
    }
    println!("differential: {count} scenes, {} diverged", failures.len());
    assert!(failures.is_empty(), "diverging seeds {failures:?}");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware"]
fn gpu_and_cpu_survive_stress_scenes() {
    let Some(mut gpu) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        println!("no adapter; skipping stress run");
        return;
    };
    let cases = [
        ("clip_depth_1000", stress::clip_depth(1000)),
        ("layer_depth_64", stress::layer_depth(64, false)),
        ("layer_blur_depth_16", stress::layer_depth(16, true)),
        ("unique_glyphs_4096", stress::unique_glyphs(4096)),
        ("unique_images_2048", stress::unique_images(2048, 16)),
        ("viewport_one_px", stress::viewport(1.0, 1.0, 1.0)),
        ("viewport_huge", stress::viewport(4096.0, 4096.0, 2.0)),
    ];
    let mut failures = Vec::new();
    for (name, scene) in cases {
        let d = compare(&mut gpu, &scene);
        println!("{name}: {d:?}");
        if diverges(&d) {
            failures.push(name);
        }
    }
    assert!(failures.is_empty(), "stress divergence {failures:?}");
}
