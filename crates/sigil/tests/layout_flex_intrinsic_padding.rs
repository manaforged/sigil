#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

use crate::layout_stub as stub;
use sigil::layout::Space;
use sigil::widgets::Widget;
use stub::{layout_rect, stub_engine};

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::*;

fn chip_builder(label: &'static str) -> sigil::kernel::NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .padding_x(Space::Sm)
                .padding_y(Space::Xxs),
        )
        .child(text(label.to_owned()))
}

#[test]
fn chip_row_is_laid_out_without_overlap() {
    let mut cx = Cx::new_root();
    let mut row_builder = container().layout(
        LayoutStyle::flex_row()
            .gap(Space::Md)
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
        let r = layout_rect(&cx, *id);
        if prev_right > 0.0 {
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
    let mut cx = Cx::new_root();
    let chip = chip_builder("NEUTRAL");
    let row = cx.add(
        container()
            .layout(
                LayoutStyle::flex_row()
                    .gap(Space::None)
                    .align_items(AlignItems::Start),
            )
            .child(chip),
    );
    let chip_id = *cx.get(row).expect("row").children().first().expect("chip");

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, row, Size::fixed(500.0, 40.0))
        .expect("compute");

    let r = layout_rect(&cx, chip_id);
    assert_eq!(
        r.width, 72.0,
        "chip width must be text(56) + padding_x(2 * 8) = 72, got {}",
        r.width,
    );
}
