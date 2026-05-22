//! W4.99.RC2 — repros for the two rendering failures the VG1 dispatch
//! had to work around but never root-caused:
//!
//! 1. Children of a [`Position::Relative`] parent didn't paint in the
//!    offscreen render even with explicit `BackgroundThemeRef` styles.
//! 2. Narrow text glyphs (a single character clipped to ≤ 14px) didn't
//!    rasterise — the offscreen capture showed zero glyph pixels.
//!
//! Each test renders a minimal tree, then samples the resulting RGBA
//! buffer for the expected non-background pixels. Both are expected to
//! FAIL before the RC2 fix and PASS after.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::print_stdout, clippy::doc_markdown, clippy::too_many_lines)]

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use kernel as _;
use layout as _;
use reactive as _;
use style as _;
use tasks as _;

use sigil::prelude::*;

mod golden_support;

use golden_support::{InteractionMode, render_widget_to_png};

/// Bug A — a deeply-nested child (here: `Position::Absolute` inside a
/// `Position::Relative` parent inside a padded root) should paint at the
/// position the layout engine assigned it. Root padding shifts the
/// middle container off (0,0), which makes the scene-emitter's
/// "double-accumulate parent_origin onto an already-root-coord layout
/// rect" bug visible — the absolute child paints at the wrong screen
/// location (off by the root padding).
#[test]
fn position_relative_child_paints() {
    // Root padding shifts the middle container to (24, 24) in root
    // coords. Without it, the bug is invisible because parent_origin=0
    // and double-counting still gives the right answer.
    const ROOT_PADDING: u32 = 24;
    // Sample point: middle.origin + inset(20,20) + half(40,40) =
    // (24+20+20, 24+20+20) = (64, 64). With the double-count bug, the
    // accent block lands at (24+24+20, 24+24+20) = (68, 68) — so a
    // sample at (64, 64) shows surface_pressed (parent track) instead
    // of accent.
    const SAMPLE_X: u32 = 64;
    const SAMPLE_Y: u32 = 64;

    let Some(img) = render_widget_to_png(
        |cx| {
            let inner = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(40.0, 40.0)
                        .position(Position::Absolute)
                        .inset_left(20.0)
                        .inset_top(20.0),
                )
                .class("bg-accent");
            let middle = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(80.0, 80.0)
                        .position(Position::Relative),
                )
                .class("bg-surface-pressed")
                .child(inner);
            let root = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(150.0, 150.0)
                        .padding(ROOT_PADDING as f32),
                )
                .class("bg-surface")
                .child(middle);
            cx.add(root)
        },
        InteractionMode::Default,
        [150, 150],
        (0.0, 0.0),
    ) else {
        println!("no GPU adapter — skipping position_relative_child_paints");
        return;
    };

    // Theme defaults (see crates/style/src/theme.rs):
    //   accent  = iris_light().step(9)  = rgba(0x5B, 0x5B, 0xD6) = (91, 91, 214)
    //   surface = slate_light().step(2) = rgba(0xF9, 0xF9, 0xFB) = (249, 249, 251)
    let px = img.get_pixel(SAMPLE_X, SAMPLE_Y);
    let [r, g, b, _a] = px.0;
    println!("position_relative_child_paints @ ({SAMPLE_X},{SAMPLE_Y}) = ({r},{g},{b})");

    // Accept any reasonable approximation of accent rgb(91, 91, 214).
    // sRGB textures + linearisation can drift a couple of units.
    let close_to_accent =
        (r as i32 - 91).abs() <= 6 && (g as i32 - 91).abs() <= 6 && (b as i32 - 214).abs() <= 6;
    assert!(
        close_to_accent,
        "expected accent ~(91,91,214) at ({SAMPLE_X},{SAMPLE_Y}), got ({r},{g},{b}). \
         Bug A: deeply-nested child paint mislocated (likely scene-emitter \
         double-accumulating parent_origin onto a root-coord layout rect)."
    );
}

/// Bug B — text rendered into a narrow (≤14 px wide) layout box inside
/// a nested wrapper should still rasterise its glyph at the wrapper's
/// screen position. The followups doc reports the rendered PNG showed
/// zero glyph pixels — the followups note that "(b)" (Position::Relative
/// children mislocated) and "(c)" (narrow text not rasterising) are the
/// same general thread. Confirming hypothesis: the glyph IS being
/// rasterised, but at the wrong offscreen location (double-counted
/// parent origin pushes it off the wrapper).
#[test]
fn narrow_text_renders_glyph() {
    // Wrap in a non-zero-offset parent so the scene-emitter's double-
    // accumulation visibly shifts the glyph. Without the outer padding,
    // the wrapper sits at (0,0), parent_origin=0, and the double-count
    // is invisible.
    const ROOT_PADDING: u32 = 24;
    // Wrapper is at root content origin = (24, 24). Glyph paints inside
    // wrapper. Sample window: x∈[24, 38), y∈[24, 40).
    const WIN_X: u32 = 24;
    const WIN_Y: u32 = 24;
    const WIN_W: u32 = 14;
    const WIN_H: u32 = 16;

    let Some(img) = render_widget_to_png(
        |cx| {
            let glyph = text("v")
                .layout(LayoutStyle::block().width(WIN_W as f32))
                .class("text-on-surface");
            let wrapper = container()
                .layout(LayoutStyle::block().fixed_size(16.0, 16.0))
                .class("rounded-sm bg-surface")
                .child(glyph);
            let root = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(200.0, 80.0)
                        .padding(ROOT_PADDING as f32),
                )
                .class("bg-surface")
                .child(wrapper);
            cx.add(root)
        },
        InteractionMode::Default,
        [200, 80],
        (0.0, 0.0),
    ) else {
        println!("no GPU adapter — skipping narrow_text_renders_glyph");
        return;
    };

    // Background is `surface` rgba(248, 250, 252). The on-surface text
    // colour is on_surface rgba(15, 23, 42). Count pixels visibly darker
    // than surface inside the expected wrapper window.
    let mut dark = 0u32;
    for y in WIN_Y..(WIN_Y + WIN_H) {
        for x in WIN_X..(WIN_X + WIN_W) {
            let px = img.get_pixel(x, y);
            let [r, g, b, _a] = px.0;
            let sum = r as u32 + g as u32 + b as u32;
            if sum < 600 {
                dark += 1;
            }
        }
    }
    println!(
        "narrow_text_renders_glyph: {dark} dark pixels in wrapper window \
         ({WIN_X},{WIN_Y})+{WIN_W}x{WIN_H}"
    );
    assert!(
        dark >= 4,
        "expected ≥4 glyph pixels in wrapper window ({WIN_X},{WIN_Y}) {WIN_W}x{WIN_H}, \
         got {dark}. Bug B: narrow text in nested wrapper failed to rasterise \
         at the expected screen location (likely the same double-count bug as Bug A)."
    );
}
