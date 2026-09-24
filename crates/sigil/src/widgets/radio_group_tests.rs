#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use crate::kernel::Cx;
use crate::reactive::{Runtime, signal};

#[test]
fn group_carries_radio_group_role() {
    Runtime::scope("radio-group-role", || {
        let mut cx = Cx::new_root();
        let selected = signal(String::from("a"));
        let id = radio_group(selected)
            .option("a", "A")
            .option("b", "B")
            .build(&mut cx);
        let node = cx.get(id).expect("group node");
        assert_eq!(node.a11y.role, crate::kernel::Role::RadioGroup);
        assert_eq!(node.children().len(), 2, "two option rows");
    });
}

#[test]
fn option_rows_carry_radio_button_role() {
    Runtime::scope("radio-group-option-role", || {
        let mut cx = Cx::new_root();
        let selected = signal(String::from("a"));
        let id = radio_group(selected).option("a", "A").build(&mut cx);
        let group = cx.get(id).expect("group node");
        let row = cx.get(group.children()[0]).expect("row node");
        assert_eq!(row.a11y.role, crate::kernel::Role::RadioButton);
    });
}

#[test]
fn defaults_to_primary_tone() {
    Runtime::scope("radio-group-default-tone", || {
        let group = radio_group(signal(String::from("a"))).option("a", "A");
        assert!(matches!(group.tone, Tone::Primary));
    });
}

#[test]
fn mark_ring_clears_non_text_contrast() {
    use crate::style::Rgba8;
    use crate::style::scale::ScaleRole;
    use crate::style::theme::Theme;

    #[allow(clippy::suboptimal_flops, reason = "intentional, reviewed trade-off")]
    fn luminance(c: Rgba8) -> f64 {
        let ch = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b)
    }

    for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        let ring = theme.scale(ScaleRole::Gray).step(9);
        let (lr, ls) = (luminance(ring), luminance(theme.surface));
        let ratio = (lr.max(ls) + 0.05) / (lr.min(ls) + 0.05);
        assert!(
            ratio >= 3.0,
            "{name}: gray-9 ring on surface measures {ratio:.2}:1, below the 3:1 non-text floor"
        );
    }
}
