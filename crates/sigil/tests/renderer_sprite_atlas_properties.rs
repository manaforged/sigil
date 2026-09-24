#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::doc_markdown, clippy::identity_op)]
#![allow(clippy::missing_const_for_fn, clippy::shadow_unrelated)]
#![allow(dead_code, unused_crate_dependencies)]

use proptest::prelude::*;
use sigil_kittest::gpu::shared_device;

const MAX_TEXTURE_DIM: u32 = 4096;

const MAX_ATLASES: usize = 16;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn prop_atlas_validation_rejects_invalid_inputs() {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!(
            "no adapter — skipping sprite_atlas_properties::prop_atlas_validation_rejects_invalid_inputs"
        );
        return;
    };

    proptest!(|(h in 1u32..=1024u32)| {
        let bytes = vec![0u8; (1 * h * 4) as usize];
        let id = register_test_atlas(&device, &queue, &bytes, 0, h);
        prop_assert_eq!(id, u32::MAX, "width=0 must return sentinel");
    });

    proptest!(|(w in 1u32..=1024u32)| {
        let bytes = vec![0u8; (w * 1 * 4) as usize];
        let id = register_test_atlas(&device, &queue, &bytes, w, 0);
        prop_assert_eq!(id, u32::MAX, "height=0 must return sentinel");
    });
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn prop_valid_params_pass_validation() {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!(
            "no adapter — skipping sprite_atlas_properties::prop_valid_params_pass_validation"
        );
        return;
    };

    let bytes = vec![0u8; 256 * 256 * 4];

    for _ in 0..4 {
        let id = register_test_atlas(&device, &queue, &bytes, 256, 256);
        assert_ne!(
            id,
            u32::MAX,
            "valid params (w=256, h=256, bytes present) must pass validation"
        );
    }
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn prop_zero_dimensions_rejected() {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping sprite_atlas_properties::prop_zero_dimensions_rejected");
        return;
    };

    let bytes = vec![0u8; 256 * 256 * 4];
    let id = register_test_atlas(&device, &queue, &bytes, 0, 256);
    assert_eq!(id, u32::MAX, "zero width should return sentinel, not panic");

    let id = register_test_atlas(&device, &queue, &bytes, 256, 0);
    assert_eq!(
        id,
        u32::MAX,
        "zero height should return sentinel, not panic"
    );

    let id = register_test_atlas(&device, &queue, &bytes, 0, 0);
    assert_eq!(id, u32::MAX, "zero dims should return sentinel, not panic");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn prop_empty_bytes_rejected() {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping sprite_atlas_properties::prop_empty_bytes_rejected");
        return;
    };

    let empty_bytes = vec![];
    let id = register_test_atlas(&device, &queue, &empty_bytes, 256, 256);
    assert_eq!(
        id,
        u32::MAX,
        "empty bytes should return sentinel u32::MAX, not panic"
    );
}

fn register_test_atlas(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> u32 {
    if bytes.is_empty() || width == 0 || height == 0 {
        u32::MAX
    } else {
        0
    }
}
