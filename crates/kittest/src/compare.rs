use std::path::{Path, PathBuf};

use crate::regen_enabled;

#[must_use]
pub fn golden_path_in(manifest_dir: &str, stem: &str) -> PathBuf {
    let mut p = PathBuf::from(manifest_dir);
    p.push("tests");
    p.push("goldens");
    p.push(format!("{stem}.png"));
    p
}

pub fn assert_png_matches_golden(actual: &image::RgbaImage, golden: &Path, max_delta_pct: f32) {
    const CHANNEL_TOLERANCE: u8 = 2;

    if regen_enabled() {
        guard_regen_does_not_erase_content(actual, golden);
        std::fs::create_dir_all(golden.parent().expect("golden parent")).expect("mkdir goldens");
        actual.save(golden).expect("write golden png");
        println!("REGEN: wrote {}", golden.display());
        return;
    }

    assert!(
        golden.exists(),
        "golden file missing: {}\n\
         Run with SIGIL_REGEN_GOLDENS=1 to create initial bakes.",
        golden.display(),
    );

    let expected = image::open(golden)
        .unwrap_or_else(|err| panic!("decode golden {}: {err}", golden.display()))
        .to_rgba8();

    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "golden dimension mismatch for {}",
        golden.display(),
    );

    let total = u64::from(actual.width()) * u64::from(actual.height());
    let mut differing: u64 = 0;
    for (a, b) in actual.pixels().zip(expected.pixels()) {
        let diff =
            a.0.iter()
                .zip(b.0.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE);
        if diff {
            differing += 1;
        }
    }
    let pct = (differing as f64 / total as f64) * 100.0;
    assert!(
        pct as f32 <= max_delta_pct,
        "golden {} differs in {:.3}% of pixels (>{} channel delta), \
         budget {:.3}%. Re-bake with SIGIL_REGEN_GOLDENS=1 if intentional.",
        golden.display(),
        pct,
        CHANNEL_TOLERANCE,
        max_delta_pct,
    );
}

fn ink(image: &image::RgbaImage) -> usize {
    use std::collections::HashMap;
    let mut census: HashMap<[u8; 4], usize> = HashMap::new();
    for px in image.pixels() {
        *census.entry(px.0).or_default() += 1;
    }
    let background = census.values().copied().max().unwrap_or(0);
    (image.width() as usize * image.height() as usize).saturating_sub(background)
}

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

pub fn assert_goldens_differ_in(manifest_dir: &str, name_a: &str, name_b: &str) {
    let path_a = golden_path_in(manifest_dir, name_a);
    let path_b = golden_path_in(manifest_dir, name_b);
    let bytes_a = std::fs::read(&path_a)
        .unwrap_or_else(|err| panic!("read golden {}: {err}", path_a.display()));
    let bytes_b = std::fs::read(&path_b)
        .unwrap_or_else(|err| panic!("read golden {}: {err}", path_b.display()));
    assert_ne!(
        bytes_a, bytes_b,
        "goldens '{name_a}' and '{name_b}' are byte-identical — the state \
         change they exist to capture renders no visible difference. Fix the \
         widget, then re-bake with SIGIL_REGEN_GOLDENS=1.",
    );
}
