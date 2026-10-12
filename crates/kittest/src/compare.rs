use std::path::{Path, PathBuf};

use crate::gpu::adapter_key;
use crate::harness::fail;
use crate::{Regen, Rendered, regen_mode};

#[must_use]
pub fn golden_path_in(manifest_dir: &str, stem: &str) -> PathBuf {
    [
        manifest_dir,
        "tests",
        "goldens",
        format!("{stem}.png").as_str(),
    ]
    .iter()
    .collect()
}

fn adapter_golden(golden: &Path) -> Option<PathBuf> {
    let key = adapter_key()?;
    let stem = golden.file_stem()?.to_str()?;
    Some(golden.with_file_name(format!("{stem}.{key}.png")))
}

#[track_caller]
#[allow(clippy::print_stdout)]
fn write_golden(actual: &image::RgbaImage, golden: &Path) {
    guard_regen_does_not_erase_content(actual, golden);
    create_parent(golden);
    actual
        .save(golden)
        .unwrap_or_else(|err| fail(format!("write golden {}: {err}", golden.display())));
    println!("REGEN: wrote {}", golden.display());
}

#[track_caller]
fn create_parent(path: &Path) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .unwrap_or_else(|err| fail(format!("create {}: {err}", dir.display())));
    }
}

#[track_caller]
fn differing_pct(actual: &image::RgbaImage, golden: &Path) -> f64 {
    let expected = image::open(golden)
        .unwrap_or_else(|err| fail(format!("decode golden {}: {err}", golden.display())))
        .to_rgba8();
    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "golden dimension mismatch for {}",
        golden.display(),
    );
    let total = u64::from(actual.width()) * u64::from(actual.height());
    let differing = actual
        .pixels()
        .zip(expected.pixels())
        .filter(|(a, b)| {
            a.0.iter()
                .zip(b.0.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    differing_percent(u64::try_from(differing).unwrap_or(u64::MAX), total)
}

const CHANNEL_TOLERANCE: u8 = 2;

#[track_caller]
#[allow(clippy::print_stdout)]
pub fn assert_rendered_matches_golden(rendered: Rendered, golden: &Path, max_delta_pct: f32) {
    let Some(img) = rendered.expect("sigil-kittest") else {
        println!("no GPU adapter — skipping {}", golden.display());
        return;
    };
    assert_png_matches_golden(&img, golden, max_delta_pct);
}

#[track_caller]
pub fn assert_png_matches_golden(actual: &image::RgbaImage, golden: &Path, max_delta_pct: f32) {
    let keyed = adapter_golden(golden);
    let existing_keyed = keyed.clone().filter(|path| path.exists());
    match regen_mode() {
        Some(Regen::All) => {
            write_golden(actual, existing_keyed.as_deref().unwrap_or(golden));
            return;
        }
        Some(Regen::Adapter) => {
            if !golden.exists() {
                write_golden(actual, golden);
            } else if differing_pct(actual, golden) > f64::from(max_delta_pct) {
                let path = keyed.unwrap_or_else(|| {
                    fail(format!("no GPU adapter key for {}", golden.display()))
                });
                write_golden(actual, &path);
            } else if let Some(path) = existing_keyed {
                std::fs::remove_file(&path)
                    .unwrap_or_else(|err| fail(format!("remove {}: {err}", path.display())));
            }
            return;
        }
        None => {}
    }

    let reference = existing_keyed.as_deref().unwrap_or(golden);
    assert!(
        reference.exists(),
        "golden file missing: {}\n\
         Run with SIGIL_REGEN_GOLDENS=1 to create initial bakes.",
        reference.display(),
    );
    let pct = differing_pct(actual, reference);
    assert!(
        pct <= f64::from(max_delta_pct),
        "golden {} differs in {:.3}% of pixels (>{} channel delta), \
         budget {:.3}%. Re-bake with SIGIL_REGEN_GOLDENS=1 if intentional, \
         or SIGIL_REGEN_GOLDENS=adapter for this GPU adapter ({}).",
        reference.display(),
        pct,
        CHANNEL_TOLERANCE,
        max_delta_pct,
        adapter_key().unwrap_or_default(),
    );
}

#[track_caller]
#[allow(clippy::print_stdout)]
pub fn assert_text_matches_golden(actual: &str, golden: &Path) {
    if regen_mode().is_some() {
        create_parent(golden);
        std::fs::write(golden, actual)
            .unwrap_or_else(|err| fail(format!("write golden {}: {err}", golden.display())));
        println!("REGEN: wrote {}", golden.display());
        return;
    }
    let expected = std::fs::read_to_string(golden).unwrap_or_else(|err| {
        fail(format!(
            "golden file {}: {err}\nRun with SIGIL_REGEN_GOLDENS=1 to create it.",
            golden.display()
        ))
    });
    assert!(
        actual == expected,
        "text golden {} differs.\n--- expected\n{expected}\n--- actual\n{actual}\n\
         Re-bake with SIGIL_REGEN_GOLDENS=1 if intentional.",
        golden.display(),
    );
}

#[allow(clippy::cast_precision_loss)]
fn differing_percent(differing: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (differing as f64 / total as f64) * 100.0
}

fn ink(image: &image::RgbaImage) -> usize {
    let mut census = std::collections::HashMap::<[u8; 4], usize>::new();
    for px in image.pixels() {
        *census.entry(px.0).or_default() += 1;
    }
    let background = census.values().copied().max().unwrap_or(0);
    image.pixels().len().saturating_sub(background)
}

#[track_caller]
fn guard_regen_does_not_erase_content(actual: &image::RgbaImage, golden: &Path) {
    const RETAINED_FRACTION: f32 = 0.5;

    if std::env::var("SIGIL_REGEN_ALLOW_CONTENT_LOSS").as_deref() == Ok("1") {
        return;
    }
    let Ok(previous) = image::open(golden) else {
        return;
    };
    let before = ink(&previous.to_rgba8());
    let after = ink(actual);
    #[allow(clippy::cast_precision_loss)]
    let retained = if before == 0 {
        1.0
    } else {
        after as f32 / before as f32
    };
    assert!(
        retained >= RETAINED_FRACTION,
        "re-bake of {} would erase {:.0}% of the drawn content \
         ({before} -> {after} non-background pixels).\n\
         A re-bake overwrites the reference, so a regression that deletes \
         content would certify itself. Look at the render before accepting \
         it; if the reduction really is intended, set \
         SIGIL_REGEN_ALLOW_CONTENT_LOSS=1.",
        golden.display(),
        (1.0 - retained) * 100.0,
    );
}

#[track_caller]
pub fn assert_goldens_differ_in(manifest_dir: &str, name_a: &str, name_b: &str) {
    let [bytes_a, bytes_b] = [name_a, name_b].map(|name| {
        let path = golden_path_in(manifest_dir, name);
        std::fs::read(&path)
            .unwrap_or_else(|err| fail(format!("read golden {}: {err}", path.display())))
    });
    assert_ne!(
        bytes_a, bytes_b,
        "goldens '{name_a}' and '{name_b}' are byte-identical — the state \
         change they exist to capture renders no visible difference. Fix the \
         widget, then re-bake with SIGIL_REGEN_GOLDENS=1.",
    );
}
