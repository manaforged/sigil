//! Repro tests for the 2026-05-19 VG1 renderer-correctness findings,
//! items (b) and (c):
//!
//!   (b) Children of `Position::Relative` parents did not paint reliably
//!       in the offscreen path (the VG1 scroll thumb had to be hoisted
//!       to a viewport-sibling absolute node as a workaround).
//!   (c) Narrow text glyphs in flex/block children did not rasterise —
//!       `text("v")` at a ~14px block width inside a 16x16 wrapper
//!       produced zero glyph pixels. Same family as the Tooltip
//!       W5.3a followup (bubble renders, inner text invisible in a
//!       nested absolute-positioned container).
//!
//! Built directly against `kernel` + `layout` + `style` + `renderer`
//! (no `sigil` dependency) so the repro stays in the renderer crate's
//! own test scope, mirroring `tests/style_integration.rs`'s pattern but
//! adding a real wgpu offscreen pixel capture (`golden_support`'s
//! pattern, factored into `child_paint_repro/support.rs`) instead of
//! only inspecting `CommandScene` contents.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::print_stdout, clippy::doc_markdown)]
#![allow(clippy::cast_precision_loss)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_icons as _;
use sigil_shell as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use sigil_kernel::{Cx, prelude::*};
use sigil_layout::{LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_style::{NodeStyleExt, Theme};

#[path = "child_paint_repro/support.rs"]
mod support;
use support::{InkWindow, capture, close, count_ink};

/// (b) — VG1 scroll-thumb shape: an out-of-flow `Position::Absolute`
/// child nested directly inside a `Position::Relative` track, which is
/// itself inside an `overflow: hidden` viewport (the exact ancestor
/// chain the scroll widget had before the thumb was hoisted to a
/// viewport-sibling). The thumb must still paint at its assigned
/// position instead of vanishing or landing off the clip.
#[test]
fn relative_track_absolute_child_paints_under_clip() {
    const VIEWPORT_W: u32 = 64;
    const VIEWPORT_H: u32 = 96;
    const THUMB_SAMPLE_X: u32 = 4;
    const THUMB_SAMPLE_Y: u32 = 20;

    let img = capture(build_relative_track_shape, [VIEWPORT_W, VIEWPORT_H]);

    let px = img.get_pixel(THUMB_SAMPLE_X, THUMB_SAMPLE_Y);
    let [r, g, b, _a] = px.0;
    let accent = Theme::default().accent;
    println!(
        "relative_track_absolute_child_paints_under_clip @ \
         ({THUMB_SAMPLE_X},{THUMB_SAMPLE_Y}) = ({r},{g},{b}), expected accent ~\
         ({},{},{})",
        accent.r, accent.g, accent.b
    );
    assert!(
        close(r, g, b, accent, 6),
        "thumb (Position::Absolute child of a Position::Relative track, under an \
         overflow:hidden viewport clip) did not paint: expected accent \
         ({},{},{}) at ({THUMB_SAMPLE_X},{THUMB_SAMPLE_Y}), got ({r},{g},{b})",
        accent.r,
        accent.g,
        accent.b,
    );
}

fn build_relative_track_shape(cx: &mut Cx) -> sigil_kernel::NodeId {
    const VIEWPORT_W: f32 = 64.0;
    const VIEWPORT_H: f32 = 96.0;

    let thumb = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(8.0, 40.0)
                .position(Position::Absolute)
                .inset_left(0.0)
                .inset_top(0.0),
        )
        .class("bg-accent");
    let track = container()
        .layout(LayoutStyle::block().fixed_size(8.0, VIEWPORT_H))
        .class("bg-surface-pressed")
        .child(thumb);
    let viewport = container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(VIEWPORT_W, VIEWPORT_H)
                .overflow_y(Overflow::Hidden),
        )
        .class("bg-surface")
        .child(track);
    cx.add(viewport)
}

#[test]
fn rounded_overflow_masks_child_pixels() {
    const SIZE: u32 = 80;
    let img = capture(build_rounded_overflow_shape, [SIZE, SIZE]);
    let accent = Theme::default().accent;
    let corner = img.get_pixel(9, 9).0;
    let edge = img.get_pixel(40, 9).0;
    let centre = img.get_pixel(40, 40).0;

    assert!(
        corner[0] < 8 && corner[1] < 8 && corner[2] < 8,
        "rounded overflow leaked its child into the corner: {corner:?}"
    );
    assert!(
        close(edge[0], edge[1], edge[2], accent, 6),
        "rounded overflow removed the top-edge midpoint: {edge:?}"
    );
    assert!(
        close(centre[0], centre[1], centre[2], accent, 6),
        "rounded overflow removed the child centre: {centre:?}"
    );
}

fn build_rounded_overflow_shape(cx: &mut Cx) -> sigil_kernel::NodeId {
    let child = container()
        .layout(LayoutStyle::block().fixed_size(64.0, 64.0))
        .class("bg-accent");
    let clipped = container()
        .layout(LayoutStyle::block().fixed_size(64.0, 64.0))
        .class("rounded-full overflow-hidden")
        .child(child);
    cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(80.0, 80.0).padding(8.0))
            .child(clipped),
    )
}

#[test]
fn rectangular_clip_preserves_child_corner_geometry() {
    let img = capture(build_partially_clipped_round_rect, [32, 32]);
    let accent = Theme::default().accent;
    let visible_top = img.get_pixel(22, 10).0;

    assert!(
        close(visible_top[0], visible_top[1], visible_top[2], accent, 8),
        "rectangular clipping reshaped the child's original rounded box: {visible_top:?}"
    );
}

fn build_partially_clipped_round_rect(cx: &mut Cx) -> sigil_kernel::NodeId {
    let child = container()
        .layout(LayoutStyle::block().fixed_size(24.0, 16.0))
        .class("rounded-full bg-accent");
    let clipped = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(16.0, 16.0)
                .overflow_x(Overflow::Hidden)
                .overflow_y(Overflow::Hidden),
        )
        .child(child);
    cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(32.0, 32.0).padding(8.0))
            .child(clipped),
    )
}

/// (c) — narrow text glyph inside a flex child. `text("v")` clipped to
/// a 14px block width, nested inside a 16x16 wrapper that is itself a
/// flex item (not the simpler block-in-block nesting), must still
/// rasterise visible glyph ink.
#[test]
fn narrow_text_in_flex_child_renders_ink() {
    const ROOT_W: u32 = 64;
    const ROOT_H: u32 = 32;
    const WIN_X: u32 = 8;
    const WIN_Y: u32 = 8;
    const WIN_W: u32 = 14;
    const WIN_H: u32 = 16;

    let img = capture(build_narrow_text_flex_shape, [ROOT_W, ROOT_H]);

    let window = InkWindow {
        x: WIN_X,
        y: WIN_Y,
        w: WIN_W,
        h: WIN_H,
    };
    let dark = count_ink(&img, window, Theme::default().surface);
    println!(
        "narrow_text_in_flex_child_renders_ink: {dark} ink pixels in window \
         ({WIN_X},{WIN_Y})+{WIN_W}x{WIN_H}"
    );
    assert!(
        dark >= 4,
        "expected >=4 glyph-ink pixels in flex-child wrapper window \
         ({WIN_X},{WIN_Y}) {WIN_W}x{WIN_H}, got {dark} — narrow text in a flex \
         child failed to rasterise"
    );
}

fn build_narrow_text_flex_shape(cx: &mut Cx) -> sigil_kernel::NodeId {
    const ROOT_W: f32 = 64.0;
    const ROOT_H: f32 = 32.0;
    const WIN_X: f32 = 8.0;
    const WIN_W: f32 = 14.0;

    let glyph = text("v")
        .layout(LayoutStyle::block().width(WIN_W))
        .class("text-on-surface");
    let wrapper = container()
        .layout(LayoutStyle::block().fixed_size(16.0, 16.0))
        .class("bg-surface")
        .child(glyph);
    let root = container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(ROOT_W, ROOT_H)
                .padding(WIN_X),
        )
        .class("bg-surface")
        .child(wrapper);
    cx.add(root)
}

/// (c continued) — the Tooltip W5.3a shape: text nested inside a
/// `Position::Absolute` bubble that is itself a child of a
/// `Position::Relative` wrapper (not the wrapper's flow child — the
/// bubble is pulled out of flow like the real tooltip). Glyph ink must
/// appear inside the bubble's painted rect.
const TOOLTIP_ROOT_W: u32 = 160;
const TOOLTIP_ROOT_H: u32 = 100;
const TOOLTIP_BUBBLE_X: u32 = 4;
const TOOLTIP_BUBBLE_Y: u32 = 28;
const TOOLTIP_BUBBLE_W: u32 = 110;
const TOOLTIP_BUBBLE_H: u32 = 26;

#[test]
fn nested_absolute_text_tooltip_shape_renders_ink() {
    let img = capture(build_tooltip_shape, [TOOLTIP_ROOT_W, TOOLTIP_ROOT_H]);

    let window = InkWindow {
        x: TOOLTIP_BUBBLE_X + 4,
        y: TOOLTIP_BUBBLE_Y + 4,
        w: TOOLTIP_BUBBLE_W - 8,
        h: TOOLTIP_BUBBLE_H - 8,
    };
    let dark = count_ink(&img, window, Theme::default().surface);
    println!(
        "nested_absolute_text_tooltip_shape_renders_ink: {dark} ink pixels in \
         bubble interior"
    );
    assert!(
        dark >= 4,
        "expected >=4 glyph-ink pixels inside the tooltip-shaped bubble \
         (Position::Absolute child of a Position::Relative wrapper), got {dark} \
         — text nested in an absolute container failed to rasterise"
    );
}

/// Tooltip W5.3a shape: a `Position::Absolute` bubble (holding a text
/// child) as a sibling of the hovered target, both children of a
/// `Position::Relative` wrapper — the bubble is pulled out of flow
/// like the real tooltip widget.
fn build_tooltip_shape(cx: &mut Cx) -> sigil_kernel::NodeId {
    let bubble_text = text("Save the file")
        .layout(LayoutStyle::block().width(TOOLTIP_BUBBLE_W as f32 - 8.0))
        .class("text-on-surface");
    let bubble = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(TOOLTIP_BUBBLE_W as f32, TOOLTIP_BUBBLE_H as f32)
                .position(Position::Absolute)
                .inset_left(TOOLTIP_BUBBLE_X as f32)
                .inset_top(TOOLTIP_BUBBLE_Y as f32),
        )
        .class("bg-surface")
        .child(bubble_text);
    let target = container()
        .layout(LayoutStyle::block().fixed_size(60.0, 20.0))
        .class("bg-surface-pressed");
    let wrapper = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(TOOLTIP_ROOT_W as f32, TOOLTIP_ROOT_H as f32)
                .position(Position::Relative),
        )
        .class("bg-surface")
        .child(target)
        .child(bubble);
    cx.add(wrapper)
}
