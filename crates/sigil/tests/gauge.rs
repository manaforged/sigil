#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown, clippy::too_many_lines, clippy::print_stdout)]

use sigil::kernel::{Cx, NodeId};
use sigil::prelude::*;
use sigil::reactive::{Owner, signal};
use sigil::style::{ColorAtom, StyleAtom, StyleBinding, Token};

use sigil_kittest::{Golden, Harness, InteractionMode};

fn reactive_slice(cx: &Cx, id: NodeId) -> &'static [StyleAtom] {
    let Some(binding) = cx.get(id).and_then(|node| node.extension::<StyleBinding>()) else {
        return &[];
    };
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}

fn only_child(cx: &Cx, id: NodeId) -> NodeId {
    let node = cx.get(id).expect("node exists");
    let children = node.children();
    assert_eq!(
        children.len(),
        1,
        "expected exactly one child, got {children:?}"
    );
    children[0]
}

fn fill_width_px(atoms: &[StyleAtom]) -> f32 {
    atoms
        .iter()
        .find_map(|atom| match atom {
            StyleAtom::Width(_, px) => Some(f32::from(*px)),
            _ => None,
        })
        .expect("fill atoms must include a Width(Px(_)) atom")
}

fn has_warn_tone(atoms: &[StyleAtom]) -> bool {
    atoms.iter().any(|atom| {
        matches!(
            atom,
            StyleAtom::BackgroundColor(ColorAtom::Token(Token::Warning9))
        )
    })
}

fn has_muted_accent_tone(atoms: &[StyleAtom]) -> bool {
    atoms.iter().any(|atom| {
        matches!(
            atom,
            StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent8))
        )
    })
}

#[test]
fn gauge_warn_tone_below_threshold() {
    Owner::scope("gauge-warn-threshold", || {
        let value = signal(0.5_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(gauge(value).max(1.0).warn(0.3));
        let fill_id = only_child(&cx, only_child(&cx, id));

        let before = reactive_slice(&cx, fill_id);
        assert!(
            has_muted_accent_tone(before),
            "0.5 / 1.0 is above the 0.3 threshold — fill starts on the muted accent-8 tone; got {before:?}"
        );
        assert!(!has_warn_tone(before));

        value.set(0.2);
        let after = reactive_slice(&cx, fill_id);
        assert!(
            has_warn_tone(after),
            "value/max below the warn threshold must switch the fill to warn-9; got {after:?}"
        );
        assert!(!has_muted_accent_tone(after));
        assert_ne!(before, after, "the resolved fill tone must change");
    });
}

#[test]
fn gauge_fill_tracks_signal() {
    Owner::scope("gauge-fill-tracks", || {
        let value = signal(0.25_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(gauge(value).max(1.0));
        let fill_id = only_child(&cx, only_child(&cx, id));

        let low_width = fill_width_px(reactive_slice(&cx, fill_id));

        value.set(0.75);
        let high_width = fill_width_px(reactive_slice(&cx, fill_id));

        assert!(
            high_width > low_width,
            "fill width must grow as the signal grows; low={low_width} high={high_width}"
        );
    });
}

#[test]
fn gauge_full() {
    let img = Harness::render(|| {
        let value = signal(0.9_f32);
        gauge(value).max(1.0).label("HP").warn(0.3)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([220, 40])
    .hit((1.0, 1.0))
    .image();
    Golden::new("gauge_full").assert(img);
}

#[test]
fn gauge_low_warn() {
    let img = Harness::render(|| {
        let value = signal(0.15_f32);
        gauge(value).max(1.0).label("HP").warn(0.3)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([220, 40])
    .hit((1.0, 1.0))
    .image();
    Golden::new("gauge_low_warn").assert(img);
}
