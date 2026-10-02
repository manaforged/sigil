#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::hal::ParagraphMetrics as _;
use sigil::kernel::NodeExtensions;
use sigil::style::TextStyle;
use sigil::text::{OwnedTextEngine, TextConfig};

const LABEL: &str = "row 4821";

#[test]
fn reattaching_an_extension_of_the_same_type_does_not_allocate() {
    let mut bag = NodeExtensions::default();
    let _ = bag.attach(1_u64);
    let info = allocation_counter::measure(|| {
        assert_eq!(bag.attach(2_u64), Some(1));
    });
    assert_eq!(info.count_total, 0);
}

#[test]
fn a_fitting_width_reuses_the_unbounded_paragraph() {
    let engine = OwnedTextEngine::new(TextConfig::default());
    let style = TextStyle::default();
    let unbounded = engine.layout_paragraph_styled(LABEL, None, &style).unwrap();
    let limit = unbounded.width() + 40.0;
    let info = allocation_counter::measure(|| {
        drop(
            engine
                .layout_paragraph_styled(LABEL, Some(limit), &style)
                .unwrap(),
        );
    });
    assert_eq!(info.count_total, 0);
}

#[test]
fn an_unwrapped_fixed_width_paragraph_serves_the_unbounded_lookup() {
    let engine = OwnedTextEngine::new(TextConfig::default());
    let style = TextStyle::default();
    drop(
        engine
            .layout_paragraph_styled(LABEL, Some(1000.0), &style)
            .unwrap(),
    );
    let info = allocation_counter::measure(|| {
        drop(engine.layout_paragraph_styled(LABEL, None, &style).unwrap());
    });
    assert_eq!(info.count_total, 0);
}

#[test]
fn laying_out_a_new_label_in_a_known_style_allocates_little() {
    let engine = OwnedTextEngine::new(TextConfig::default());
    let style = TextStyle::default();
    drop(
        engine
            .layout_paragraph_styled("row 1", Some(400.0), &style)
            .unwrap(),
    );
    let info = allocation_counter::measure(|| {
        drop(
            engine
                .layout_paragraph_styled("row 2", Some(400.0), &style)
                .unwrap(),
        );
    });
    assert!(info.count_total <= 8, "{} allocations", info.count_total);
}
