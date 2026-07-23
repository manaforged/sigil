//! `gauge` + `turn_strip` — state-machine and visual-golden tests for
//! the two battler HUD display widgets (HD-2D Task 10).
//!
//! State-machine tests read the resolved reactive style slice straight
//! off each node's [`StyleBinding`] extension — the same pattern
//! `phase_c_reactive_attrs.rs` uses for `text_field`/`button` reactive
//! overlays — rather than rendering, since the assertion is "did the
//! atom list change", not "does it look right" (goldens cover that).
//!
//! Goldens reuse `golden_support` directly, mirroring `command_menu.rs`.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown, clippy::too_many_lines, clippy::print_stdout)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use tiny_skia as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_kernel::{Cx, NodeId};
use sigil_reactive::{Runtime, signal};
use sigil_style::{Dimension, Px, StyleAtom, StyleBinding};

mod golden_support;

use golden_support::{
    InteractionMode, assert_png_matches_golden, golden_path, render_widget_to_png,
};

/// Read the current reactive style slices off `id`'s [`StyleBinding`]
/// extension, concatenated in install order (T0 composition) — mirrors
/// `phase_c_reactive_attrs.rs`'s helper of the same name.
fn reactive_slice(cx: &Cx, id: NodeId) -> &'static [StyleAtom] {
    let Some(binding) = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
    else {
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

fn fill_width_px(atoms: &[StyleAtom]) -> i16 {
    atoms
        .iter()
        .find_map(|atom| match atom {
            StyleAtom::Width(Dimension::Px(Px(px))) => Some(*px),
            _ => None,
        })
        .expect("fill atoms must include a Width(Px(_)) atom")
}

fn has_warn_tone(atoms: &[StyleAtom]) -> bool {
    atoms
        .iter()
        .any(|atom| matches!(atom, StyleAtom::BackgroundThemeRef("warn-9")))
}

fn has_accent_tone(atoms: &[StyleAtom]) -> bool {
    atoms
        .iter()
        .any(|atom| matches!(atom, StyleAtom::BackgroundThemeRef("accent")))
}

// ---- gauge: state machine -----------------------------------------------------

#[test]
fn gauge_warn_tone_below_threshold() {
    Runtime::scope("gauge-warn-threshold", || {
        let value = signal(0.5_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(gauge(value).max(1.0).warn(0.3));
        let fill_id = only_child(&cx, only_child(&cx, id));

        let before = reactive_slice(&cx, fill_id);
        assert!(
            has_accent_tone(before),
            "0.5 / 1.0 is above the 0.3 threshold — fill starts on the accent tone; got {before:?}"
        );
        assert!(!has_warn_tone(before));

        // Drop below the threshold: 0.2 / 1.0 = 0.2 < 0.3.
        value.set(0.2);
        let after = reactive_slice(&cx, fill_id);
        assert!(
            has_warn_tone(after),
            "value/max below the warn threshold must switch the fill to warn-9; got {after:?}"
        );
        assert!(!has_accent_tone(after));
        assert_ne!(before, after, "the resolved fill tone must change");
    });
}

#[test]
fn gauge_fill_tracks_signal() {
    Runtime::scope("gauge-fill-tracks", || {
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

// ---- turn_strip: state machine -------------------------------------------------

#[test]
fn turn_strip_active_follows_signal() {
    Runtime::scope("turn-strip-active", || {
        let entries = signal(battler_entries());
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(turn_strip(entries).active(active));
        cx.flush_reactive_children();

        let chips: Vec<NodeId> = cx.get(root).expect("turn strip root").children().to_vec();
        assert_eq!(chips.len(), 3, "one chip per entry");

        assert!(
            has_accent_tone(reactive_slice(&cx, chips[0])),
            "index 0 starts as the active chip"
        );
        assert!(!has_accent_tone(reactive_slice(&cx, chips[1])));
        assert!(!has_accent_tone(reactive_slice(&cx, chips[2])));

        active.set(2);
        assert!(
            !has_accent_tone(reactive_slice(&cx, chips[0])),
            "the old active chip must lose the highlight"
        );
        assert!(
            has_accent_tone(reactive_slice(&cx, chips[2])),
            "moving the active signal must highlight the new chip"
        );
    });
}

// ---- goldens ------------------------------------------------------------------

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
}

/// Short unit names — the golden harness's `StubMeasure` reports a
/// fixed 40x16px per text node regardless of content, so a label
/// longer than that wraps and clips within its own chip. Keep entries
/// short enough to render on one line under that stub.
fn battler_entries() -> Vec<String> {
    vec!["Alric".to_owned(), "Vex".to_owned(), "Doom".to_owned()]
}

#[test]
fn gauge_full() {
    let img = render_widget_to_png(
        |cx| {
            let value = signal(0.9_f32);
            cx.add(gauge(value).max(1.0).label("HP").warn(0.3))
        },
        InteractionMode::Default,
        [220, 40],
        (1.0, 1.0),
    );
    check("gauge_full", img);
}

#[test]
fn gauge_low_warn() {
    let img = render_widget_to_png(
        |cx| {
            let value = signal(0.15_f32);
            cx.add(gauge(value).max(1.0).label("HP").warn(0.3))
        },
        InteractionMode::Default,
        [220, 40],
        (1.0, 1.0),
    );
    check("gauge_low_warn", img);
}

#[test]
fn turn_strip_default() {
    let img = render_widget_to_png(
        |cx| {
            let entries = signal(battler_entries());
            let active = signal(0_usize);
            cx.add(turn_strip(entries).active(active))
        },
        InteractionMode::Default,
        [320, 48],
        (1.0, 1.0),
    );
    check("turn_strip_default", img);
}

#[test]
fn turn_strip_active_highlight() {
    let img = render_widget_to_png(
        |cx| {
            let entries = signal(battler_entries());
            let active = signal(2_usize);
            cx.add(turn_strip(entries).active(active))
        },
        InteractionMode::Default,
        [320, 48],
        (1.0, 1.0),
    );
    check("turn_strip_active_highlight", img);
}
