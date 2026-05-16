#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;
use taffy as _;
mod stub;
use stub::stub_engine;

use sigil_kernel::{TextContent, prelude::*};
use sigil_layout::prelude::*;
use taffy::prelude::{AvailableSpace, Size as TaffySize};

struct FixedMeasure;

impl TextMeasure for FixedMeasure {
    fn measure(&self, _source: &TextContent, _space: TaffySize<AvailableSpace>) -> TaffySize<f32> {
        TaffySize {
            width: 100.0,
            height: 20.0,
        }
    }
}

#[test]
fn custom_text_measure_overrides_placeholder_measurement() {
    let default_width = computed_text_width(stub_engine());
    let custom_width = computed_text_width(LayoutEngine::with_text_measure(Box::new(FixedMeasure)));

    assert_eq!(default_width, 16.0);
    assert_eq!(custom_width, 100.0);
}

fn computed_text_width(mut engine: LayoutEngine) -> f32 {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("AB")));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().align_items(AlignItems::FlexStart));

    let child = cx
        .get(root)
        .and_then(|node| node.children.first())
        .copied()
        .expect("child exists");

    engine
        .compute(&mut cx, root, Size::max_content())
        .expect("compute succeeds");

    cx.get(child)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("text result")
        .rect()
        .width
}
