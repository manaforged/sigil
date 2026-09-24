#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::{Cx, IntoNode, NodeType};
use crate::reactive::Runtime;

#[test]
fn chip_uses_label_role_and_holds_text_child() {
    Runtime::scope("chip-build-static", || {
        let mut cx = Cx::new_root();
        let id = chip("INFO", Tone::Info).build(&mut cx);
        let node = cx.get(id).expect("chip node exists");
        assert_eq!(node.a11y.role, crate::kernel::Role::Label);
        assert_eq!(node.children().len(), 1, "chip wraps a single text child");
        let text_child = cx.get(node.children()[0]).expect("chip text child exists");
        assert!(
            matches!(text_child.kind, NodeType::Text { .. }),
            "chip child is the static label text node"
        );
    });
}

#[test]
fn owned_label_reaches_text_child() {
    Runtime::scope("chip-build-owned", || {
        let mut cx = Cx::new_root();
        let id = chip(String::from("ERROR"), Tone::Error).build(&mut cx);
        let node = cx.get(id).expect("chip node exists");
        assert_eq!(node.children().len(), 1, "chip wraps one text child");
        let text_child = cx.get(node.children()[0]).expect("chip text child exists");
        match &text_child.kind {
            NodeType::Text { content, .. } => {
                assert_eq!(
                    content.as_static_str(),
                    Some("ERROR"),
                    "owned label String reaches the text child verbatim"
                );
            }
            other => panic!("expected NodeType::Text, got {other:?}"),
        }
    });
}

#[test]
fn tone_class_strings_are_distinct() {
    let tones = [
        Tone::Neutral,
        Tone::Primary,
        Tone::Info,
        Tone::Success,
        Tone::Warn,
        Tone::Error,
    ];
    for (i, a) in tones.iter().enumerate() {
        for b in &tones[i + 1..] {
            assert_ne!(
                chip_class(*a),
                chip_class(*b),
                "{a:?} and {b:?} class collide"
            );
        }
    }
}

#[test]
fn every_tone_clears_wcag_aa_contrast() {
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

    let theme = Theme::light();
    let cases = [
        (Tone::Neutral, ScaleRole::Gray, 12),
        (Tone::Primary, ScaleRole::Accent, 12),
        (Tone::Info, ScaleRole::Info, 12),
        (Tone::Success, ScaleRole::Success, 12),
        (Tone::Warn, ScaleRole::Warn, 12),
        (Tone::Error, ScaleRole::Error, 11),
    ];
    for (tone, role, text_step) in cases {
        let scale = theme.scale(role);
        let (bg, fg) = (scale.step(3), scale.step(text_step));
        let (lb, lf) = (luminance(bg), luminance(fg));
        let ratio = (lb.max(lf) + 0.05) / (lb.min(lf) + 0.05);
        assert!(
            ratio >= 4.5,
            "{tone:?}: step-{text_step} on step-3 measures {ratio:.2}:1, below WCAG AA 4.5:1"
        );
    }
}

#[test]
fn badge_is_a_pill_chip() {
    Runtime::scope("badge-pill", || {
        let mut cx = Cx::new_root();
        let id = badge("NEW", Tone::Primary).build(&mut cx);
        let node = cx.get(id).expect("badge node exists");
        assert_eq!(node.a11y.role, crate::kernel::Role::Label);
        assert_eq!(node.children().len(), 1);
    });
}

#[test]
fn activate_promotes_the_chip_to_a_button() {
    Runtime::scope("chip-activate-role", || {
        let mut cx = Cx::new_root();
        let id = chip("Done", Tone::Success).activate(|| {}).build(&mut cx);
        let node = cx.get(id).expect("chip node exists");
        assert_eq!(node.a11y.role, crate::kernel::Role::Button);
        assert!(node.a11y.focusable, "an activatable chip takes focus");
    });
}

#[test]
fn every_tone_renders_one_text_child() {
    Runtime::scope("chip-build-all-tones", || {
        let mut cx = Cx::new_root();
        for tone in [
            Tone::Neutral,
            Tone::Primary,
            Tone::Info,
            Tone::Success,
            Tone::Warn,
            Tone::Error,
        ] {
            let id = chip("x", tone).build(&mut cx);
            let node = cx.get(id).expect("chip node exists");
            assert_eq!(
                node.children().len(),
                1,
                "{tone:?} chip should wrap exactly one text child"
            );
        }
    });
}
