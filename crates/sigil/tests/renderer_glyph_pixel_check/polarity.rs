use super::*;

const POLARITY_TOLERANCE: f32 = 0.15;

const POLARITY_TEXT: &str = "sample";

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn polarity_stroke_weight_parity_scale_1_0() {
    assert_polarity_parity(1.0);
}
#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn polarity_stroke_weight_parity_scale_2_0() {
    assert_polarity_parity(2.0);
}

fn assert_polarity_parity(scale: f32) {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping polarity parity at scale {scale}");
        return;
    };
    let (lod_px, lw, lh) = render_text_string(
        &device,
        &queue,
        scale,
        POLARITY_TEXT,
        (sigil::style::Color::WHITE, wgpu::Color::BLACK),
    );
    let lod = measure(&lod_px, lw, lh);
    let (dol_raw, dw, dh) = render_text_string(
        &device,
        &queue,
        scale,
        POLARITY_TEXT,
        (sigil::style::Color::BLACK, wgpu::Color::WHITE),
    );
    let dol_px: Vec<u8> = dol_raw.iter().map(|b| 255 - b).collect();
    let dol = measure(&dol_px, dw, dh);

    let mass_rel = rel_diff(lod.ink_mass, dol.ink_mass);
    let ink_rel = rel_diff(lod.ink_fraction, dol.ink_fraction);
    println!(
        "polarity scale {scale}: LOD mass={:.5} ink={:.5} | DOL mass={:.5} ink={:.5} | \
         mass_rel={:.3} ink_rel={:.3}",
        lod.ink_mass, lod.ink_fraction, dol.ink_mass, dol.ink_fraction, mass_rel, ink_rel,
    );

    assert!(
        mass_rel <= POLARITY_TOLERANCE,
        "scale {scale}: stroke-weight (ink_mass) parity {:.1}% exceeds {:.0}% budget — \
         LOD {:.5} vs DOL {:.5}. Re-tune gamma_dark_on_light in slug/tuning.rs.",
        mass_rel * 100.0,
        POLARITY_TOLERANCE * 100.0,
        lod.ink_mass,
        dol.ink_mass,
    );
    assert!(
        ink_rel <= POLARITY_TOLERANCE,
        "scale {scale}: stroke-weight (ink_fraction) parity {:.1}% exceeds {:.0}% budget — \
         LOD {:.5} vs DOL {:.5}.",
        ink_rel * 100.0,
        POLARITY_TOLERANCE * 100.0,
        lod.ink_fraction,
        dol.ink_fraction,
    );
}
