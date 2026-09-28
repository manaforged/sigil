#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]

use sigil_kittest::parity::{check_cpu, minimize};
use sigil_kittest::scene_gen::{GeneratedScene, fixture_text};

fn seeds() -> std::ops::Range<u64> {
    let start = std::env::var("SIGIL_SWEEP_START")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let count: u64 = std::env::var("SIGIL_SWEEP_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64);
    start..start + count
}

fn fails(scene: &GeneratedScene) -> Option<String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_cpu(scene))) {
        Ok(Ok(())) => None,
        Ok(Err(message)) => Some(message),
        Err(payload) => Some(format!(
            "panic: {}",
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default()
        )),
    }
}

#[test]
fn cpu_renders_generated_scenes_deterministically() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut failures = Vec::new();
    for seed in seeds() {
        let scene = GeneratedScene::from_seed(seed);
        let Some(message) = fails(&scene) else {
            continue;
        };
        let keep = minimize(scene.commands.len(), |keep| {
            fails(&scene.keep(keep)).is_some()
        });
        let small = scene.keep(&keep);
        println!(
            "seed {seed}: {message}\n{}viewport {:?} scale {}\n{:#?}",
            fixture_text(seed, &keep),
            small.viewport,
            small.scale,
            small.commands
        );
        failures.push(seed);
    }
    assert!(failures.is_empty(), "failing seeds {failures:?}");
}

#[test]
fn saved_regression_scenes_render_cleanly() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scenes");
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .expect("fixture dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no fixtures in {}", dir.display());
    for path in names {
        let text = std::fs::read_to_string(&path).expect("fixture text");
        let scene = GeneratedScene::from_fixture(&text).expect("valid fixture");
        let failure = fails(&scene);
        assert!(failure.is_none(), "{}: {failure:?}", path.display());
    }
}
