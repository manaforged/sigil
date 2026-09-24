#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::stub_engine;

use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;

fn rect(cx: &Cx, id: NodeId) -> sigil::hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("rect")
        .rect()
}

#[test]
fn non_grow_row_beside_grow_spacer_stays_intrinsic_height() {
    let mut cx = Cx::new_root();
    let card = container()
        .layout(LayoutStyle::flex_column().width(240.0))
        .child(container().layout(LayoutStyle::block().fixed_size(200.0, 80.0)));
    let row = container()
        .layout(LayoutStyle::flex_row().align_items(AlignItems::Start))
        .child(card);
    let spacer = container().layout(LayoutStyle::flex_column().grow(1.0));
    let body = container()
        .layout(LayoutStyle::flex_column().grow(1.0))
        .child(row)
        .child(spacer);
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(1408.0, 750.0))
            .child(body),
    );

    let body_id = *cx.get(root).unwrap().children().first().unwrap();
    let row_id = *cx.get(body_id).unwrap().children().first().unwrap();
    let spacer_id = *cx.get(body_id).unwrap().children().get(1).unwrap();
    let card_id = *cx.get(row_id).unwrap().children().first().unwrap();

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(1408.0, 750.0))
        .expect("compute");

    let row_rect = rect(&cx, row_id);
    let card_rect = rect(&cx, card_id);
    let spacer_rect = rect(&cx, spacer_id);

    assert_eq!(
        row_rect.height, 80.0,
        "the cards row must stay intrinsic-height (80); got {}",
        row_rect.height,
    );
    assert_eq!(
        card_rect.height, 80.0,
        "the stage card must not cross-stretch past its content (80); got {}",
        card_rect.height,
    );
    assert!(
        spacer_rect.height >= 600.0,
        "the grow-spacer must absorb the remaining vertical space; got {}",
        spacer_rect.height,
    );
}
