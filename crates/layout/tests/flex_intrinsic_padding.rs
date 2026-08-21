#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

//! Regression: `container_intrinsic_main` must NOT double-count the
//! leading padding of an auto-width flex item.
//!
//! Reproducer is the chip-row composition from `widget-playground`:
//! 5 chips (`flex_row` + `padding_x(8)` + text child) inside a `flex_row`
//! parent with `gap(12)`. Before the fix, every chip's basis came
//! back as `text_w + 24` (left padding counted in both the hypothetical
//! layout's child position AND the trailing `chrome_main` add), which
//! over-sized each chip — the row then squeezed chips toward the
//! start when total exceeded the row, producing the visible "NEUTRAL
//! INFO SUCCESS WARN ERROR" overlap.
//!
//! Post-fix: each chip's box width must equal `text_w + 16` and
//! consecutive chips must sit `gap(12)` apart with no overlap.

mod stub;
use stub::stub_engine;

use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

fn child_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("child result")
        .rect()
}

fn chip_builder(label: &'static str) -> sigil_kernel::NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .padding_x(LengthPercentage::Px(8.0))
                .padding_y(LengthPercentage::Px(2.0)),
        )
        .child(text(label.to_owned()))
}

#[test]
fn chip_row_is_laid_out_without_overlap() {
    let mut cx = Cx::new_root();
    let mut row_builder = container().layout(
        LayoutStyle::flex_row()
            .gap(LengthPercentage::Px(12.0))
            .align_items(AlignItems::Start),
    );
    for label in ["NEUTRAL", "INFO", "SUCCESS", "WARN", "ERROR"] {
        row_builder = row_builder.child(chip_builder(label));
    }
    let row = cx.add(row_builder);
    let chips: Vec<NodeId> = cx.get(row).expect("row").children().to_vec();

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, row, Size::fixed(700.0, 40.0))
        .expect("compute");

    let mut prev_right = 0.0_f32;
    let mut last_label = "";
    for (label, id) in ["NEUTRAL", "INFO", "SUCCESS", "WARN", "ERROR"]
        .iter()
        .zip(chips.iter())
    {
        let r = child_rect(&cx, *id);
        if prev_right > 0.0 {
            // Allow 0.5px float wiggle below the 12px gap.
            assert!(
                r.origin.x >= prev_right + 11.5,
                "{label}.x ({}) must be \u{2265} {last_label}.right ({}) + gap(12) \u{2014} {label} \
                 overlaps the previous chip",
                r.origin.x,
                prev_right,
            );
        }
        prev_right = r.origin.x + r.width;
        last_label = label;
    }
}

#[test]
fn container_intrinsic_main_does_not_double_count_padding() {
    // A single auto-width flex_row item with padding_x(8) and a text
    // child must report a hypothetical width = text_intrinsic + 16,
    // NOT text_intrinsic + 24. Verified by checking that the laid-out
    // box matches the leaf's intrinsic + horizontal padding only.
    let mut cx = Cx::new_root();
    let chip = chip_builder("NEUTRAL");
    let row = cx.add(
        container()
            .layout(
                LayoutStyle::flex_row()
                    .gap(LengthPercentage::Px(0.0))
                    .align_items(AlignItems::Start),
            )
            .child(chip),
    );
    let chip_id = *cx.get(row).expect("row").children().first().expect("chip");

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, row, Size::fixed(500.0, 40.0))
        .expect("compute");

    // Stub measure assigns each char 8.0 px (see crates/layout/tests/stub).
    // "NEUTRAL" is 7 chars → 56 px text width, + 16 chrome = 72 px chip box.
    let r = child_rect(&cx, chip_id);
    assert_eq!(
        r.width, 72.0,
        "chip width must be text(56) + padding_x(2 * 8) = 72, got {}",
        r.width,
    );
}
