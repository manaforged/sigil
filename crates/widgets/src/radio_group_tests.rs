#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;
use sigil_kernel::Cx;
use sigil_reactive::{Runtime, signal};

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
        assert_eq!(node.a11y.role, sigil_kernel::Role::RadioGroup);
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
        assert_eq!(row.a11y.role, sigil_kernel::Role::RadioButton);
    });
}

#[test]
fn defaults_to_primary_tone() {
    // The default tone is `Tone::Primary` so the selected mark
    // paints with the accent color out of the box. Changing the
    // default would silently re-tint every existing call site.
    Runtime::scope("radio-group-default-tone", || {
        let group = radio_group(signal(String::from("a"))).option("a", "A");
        assert!(matches!(group.tone, Tone::Primary));
    });
}

#[test]
fn mark_ring_clears_non_text_contrast() {
    use sigil_style::Rgba8;
    use sigil_style::scale::ScaleRole;
    use sigil_style::theme::Theme;

    // keep luminance arithmetic bit-stable for goldens
    #[allow(clippy::suboptimal_flops)]
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

    // The `bg-gray-9` ring must clear the WCAG 1.4.11 non-text
    // 3:1 floor against the surface it floats on, in both themes.
    // The previous flat gray-7 mark measured 1.49:1 on light
    // (Fable visual adversarial 2026-07-13).
    for (name, theme) in [("light", Theme::default()), ("dark", Theme::dark())] {
        let ring = theme.scale(ScaleRole::Gray).step(9);
        let (lr, ls) = (luminance(ring), luminance(theme.surface));
        let ratio = (lr.max(ls) + 0.05) / (lr.min(ls) + 0.05);
        assert!(
            ratio >= 3.0,
            "{name}: gray-9 ring on surface measures {ratio:.2}:1, below the 3:1 non-text floor"
        );
    }
}
