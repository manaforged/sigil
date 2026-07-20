//! Committed-PNG visual goldens for sigil widgets.
//!
//! Each (widget, state) combo renders through the wgpu offscreen path and
//! asserts byte-equivalent (within a 0.5% pixel-delta budget) against a
//! committed PNG at `crates/sigil/tests/goldens/`. Run with
//! `SIGIL_REGEN_GOLDENS=1` to re-bake — and always eyeball every regenerated
//! file in an image viewer before committing.
//!
//! Skips cleanly when no wgpu adapter is available (CI without GPU).

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use arboard as _;
use criterion as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;

use sigil::prelude::*;

mod golden_support;

use golden_support::{
    InteractionMode, assert_goldens_differ, assert_png_matches_golden, golden_path,
    render_widget_to_png, render_widget_to_png_focus_descendant,
};

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
}

/// Wrap a widget in a frame-filling, BLACK-background container with a
/// few px of padding. The quality gate measures coverage as "differs
/// from the corner pixel"; the black border keeps the corner at the
/// clear colour while the bright themed widget (and its `bg-surface`
/// backing, added at the call site) fills the rest — so coverage
/// reflects the widget, not intra-widget variance. A panel that bleeds
/// to the edge would make the corner part of the widget and collapse the
/// metric to ~0 %.
fn on_panel(inner: impl IntoNode + 'static, w: f32, h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(w, h)
                .padding(6.0)
                .align_items(AlignItems::Stretch),
        )
        .child(inner)
}

// ---- BUTTON ------------------------------------------------------------------

#[test]
fn button_default() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Default,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_default", img);
}

#[test]
fn button_hover() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Hover,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_hover", img);
}

#[test]
fn button_pressed() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Pressed,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_pressed", img);
}

#[test]
fn button_focused() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Focused,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_focused", img);
}

#[test]
fn button_disabled() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save").enabled(false)),
        InteractionMode::Disabled,
        [160, 56],
        (74.0, 22.0),
    );
    check("button_disabled", img);
}

// ---- CHECKBOX ----------------------------------------------------------------

#[test]
fn checkbox_default() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Default,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_default", img);
}

#[test]
fn checkbox_hover() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Hover,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_hover", img);
}

#[test]
fn checkbox_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Pressed,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_pressed", img);
}

#[test]
fn checkbox_focused() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Focused,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_focused", img);
}

#[test]
fn checkbox_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            let dis = signal(true);
            cx.add(
                checkbox(checked)
                    .class("bg-surface rounded-sm")
                    .disabled(dis),
            )
        },
        InteractionMode::Disabled,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_disabled", img);
}

// ---- SLIDER ------------------------------------------------------------------

#[test]
fn slider_default() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Default,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_default", img);
}

#[test]
fn slider_hover() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Hover,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_hover", img);
}

#[test]
fn slider_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Pressed,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_pressed", img);
}

#[test]
fn slider_focused() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Focused,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_focused", img);
}

#[test]
fn slider_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            let dis = signal(true);
            cx.add(slider(v, 0.0..=100.0).disabled(dis))
        },
        InteractionMode::Disabled,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_disabled", img);
}

// ---- PANEL -------------------------------------------------------------------
//
// Panel is a passive surface — it has no hover / pressed / focused / disabled
// states. We commit a single default golden.

#[test]
fn panel_default() {
    let img = render_widget_to_png(
        |cx| cx.add(panel().title("Settings")),
        InteractionMode::Default,
        [340, 92],
        (1.0, 1.0),
    );
    check("panel_default", img);
}

// ---- PROGRESS ----------------------------------------------------------------
//
// Progress is also passive — commit one golden per visually distinct fill
// level: empty, half, full.

#[test]
fn progress_empty() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(0.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_empty", img);
}

#[test]
fn progress_half() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_half", img);
}

#[test]
fn progress_full() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(100.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_full", img);
}

// ---- DROPDOWN ----------------------------------------------------------------

fn dropdown_options() -> Vec<String> {
    vec!["Easy".into(), "Normal".into(), "Hard".into()]
}

#[test]
fn dropdown_default() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Default,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_default", img);
}

#[test]
fn dropdown_hover() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Hover,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_hover", img);
}

#[test]
fn dropdown_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Pressed,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_pressed", img);
}

#[test]
fn dropdown_focused() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Focused,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_focused", img);
}

#[test]
fn dropdown_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            let dis = signal(true);
            cx.add(dropdown(sel, dropdown_options()).disabled(dis))
        },
        InteractionMode::Disabled,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_disabled", img);
}

// ---- SCROLL ------------------------------------------------------------------
//
// Scroll wraps a content node; the only meaningful golden is the default
// composed view.

#[test]
fn scroll_default() {
    let img = render_widget_to_png(
        |cx| {
            let body = container()
                .class("bg-accent rounded-sm")
                .layout(LayoutStyle::block().fixed_size(312.0, 400.0));
            cx.add(scroll(body, 400.0).class("rounded-sm"))
        },
        InteractionMode::Default,
        [330, 252],
        (1.0, 1.0),
    );
    check("scroll_default", img);
}

// ---- TEXT_FIELD --------------------------------------------------------------

#[test]
fn text_field_default() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Default,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_default", img);
}

#[test]
fn text_field_hover() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Hover,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_hover", img);
}

#[test]
fn text_field_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Pressed,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_pressed", img);
}

#[test]
fn text_field_focused() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Focused,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_focused", img);
}

#[test]
fn text_field_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            let dis = signal(true);
            cx.add(text_field(v).disabled(dis))
        },
        InteractionMode::Disabled,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_disabled", img);
}

// ---- TABS --------------------------------------------------------------------

fn tab_labels() -> Vec<String> {
    vec!["First".into(), "Second".into(), "Third".into()]
}

#[test]
fn tabs_default() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Default,
        [260, 56],
        (40.0, 17.0),
    );
    check("tabs_default", img);
}

#[test]
fn tabs_hover_tab2() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Hover,
        [260, 56],
        // Hit-point in middle of the second tab (x ∈ [80, 160]).
        (120.0, 17.0),
    );
    check("tabs_hover_tab2", img);
}

#[test]
fn tabs_active_tab2() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(1_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Default,
        [260, 56],
        (120.0, 17.0),
    );
    check("tabs_active_tab2", img);
}

#[test]
fn tabs_focused() {
    // `render_widget_to_png_focus_descendant`, not `render_widget_to_png`:
    // the tabs root is a non-focusable `Role::TabList` wrapper — routing
    // focus there fires no `focus_in` handler on anything. The individual
    // `Role::Tab` children are the focusable primitives (see
    // `crate::interaction::InteractionState::install` in
    // `widgets/tabs/builder.rs`), matching the same root-mismatch bug
    // `modal_focused` had. See
    // BUG-2026-07-02-modal-tabs-focus-states-render-nothing.
    let img = render_widget_to_png_focus_descendant(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Focused,
        [260, 56],
        (40.0, 17.0),
    );
    check("tabs_focused", img);
}

#[test]
fn tabs_focused_differs_from_default() {
    assert_goldens_differ("tabs_focused", "tabs_default");
}

#[test]
fn tabs_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()).disabled(&[1]))
        },
        InteractionMode::Default,
        [260, 56],
        (40.0, 17.0),
    );
    check("tabs_disabled", img);
}

// ---- MODAL -------------------------------------------------------------------
//
// Modal renders a full-viewport dimmed backdrop with a centred dialog
// frame inside the 400×280 golden viewport. The dialog body holds a
// regular flex child (a button) — per the rendering caveat, text inside
// extra Position::Absolute wrappers does not rasterise, so the dialog
// itself is absolute but its body children are normal flex siblings.

#[test]
fn modal_open() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    modal()
                        .title("Confirm action")
                        .show(open)
                        .child(button("OK")),
                );
            cx.add(on_panel(m, 286.0, 164.0))
        },
        InteractionMode::Default,
        // The modal's dialog is a bright panel but its backdrop region is
        // black; a `bg-surface` backing makes the whole frame interior
        // bright so the dialog reads against the thin black `on_panel`
        // border (corner = clear colour), clearing the ≥70 % rule.
        [286, 164],
        (143.0, 82.0),
    );
    check("modal_open", img);
}

#[test]
fn modal_dimmed_backdrop() {
    // The viewport clear colour is black; the modal's backdrop paints
    // 60 % opacity black over the same backing, which makes the scrim
    // invisible against a default-clear canvas. To make the dim layer
    // observable we stack an absolutely-positioned accent fill sibling
    // BEFORE the modal so both nodes occupy the same logical rect; the
    // modal's backdrop then alpha-blends over the saturated accent
    // colour, producing a measurable per-channel delta that pixel diff
    // (and the eye) can read. The wrapper itself uses block layout but
    // both its children are Position::Absolute so the modal's root
    // does not push the fill out of the viewport.
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let bg = container().class("bg-accent").layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_top(0.0)
                    .inset_left(0.0)
                    .fixed_size(400.0, 280.0),
            );
            let dialog = modal()
                .title("Confirm action")
                .show(open)
                .child(button("OK"));
            let stack = container()
                .layout(LayoutStyle::block().fixed_size(400.0, 280.0))
                .child(bg)
                .child(dialog);
            cx.add(on_panel(stack, 412.0, 292.0))
        },
        InteractionMode::Default,
        // `on_panel` adds a 6 px black border so the corner is the clear
        // colour; the saturated accent fill under the dimmed scrim then
        // dominates ≥70 % of the frame.
        [412, 292],
        (200.0, 140.0),
    );
    check("modal_dimmed_backdrop", img);
}

#[test]
fn modal_focused() {
    // `render_widget_to_png_focus_descendant`, not `render_widget_to_png`:
    // the tree root here is the `on_panel` coverage backing, not the
    // modal's own focusable dialog frame — plain `InteractionMode::Focused`
    // would set keyboard focus on the (non-focusable) panel wrapper and
    // never reach the dialog at all. The descendant-aware variant finds
    // the dialog (first focusable node in document order) and focuses
    // that instead. See BUG-2026-07-02-modal-tabs-focus-states-render-nothing.
    let img = render_widget_to_png_focus_descendant(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    modal()
                        .title("Confirm action")
                        .show(open)
                        .child(button("OK")),
                );
            cx.add(on_panel(m, 286.0, 164.0))
        },
        InteractionMode::Focused,
        [286, 164],
        // Hit point inside the dialog's OK button (unused by Focused
        // mode itself, kept for signature parity with the other modes).
        (143.0, 82.0),
    );
    check("modal_focused", img);
}

#[test]
fn modal_focused_differs_from_open() {
    // Ghost-pass guard: a focus-visible golden that renders identical to
    // its baseline state means the focus indicator authored no visible
    // pixels — WCAG 2.4.7 gap that a plain pixel-diff `check()` can't
    // catch on its own. See BUG-2026-07-02-modal-tabs-focus-states-render-nothing.
    assert_goldens_differ("modal_focused", "modal_open");
}

#[test]
fn modal_dismissing() {
    // The full dismiss-transition curve is owned by the transition
    // seam; emulating "mid-dismiss" here uses the Visual::opacity
    // styled-trait method to render the dialog at 50 % opacity over
    // the backdrop. This keeps the test in pure pixel-diff territory
    // without depending on HoverAfterFrames timing (the open signal is
    // a boolean, not a numeric the runtime can interpolate).
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    modal()
                        .title("Confirm action")
                        .show(open)
                        .child(button("OK"))
                        .opacity(0.5),
                );
            cx.add(on_panel(m, 286.0, 164.0))
        },
        InteractionMode::Default,
        [286, 164],
        (143.0, 82.0),
    );
    check("modal_dismissing", img);
}

// ---- MENU --------------------------------------------------------------------
//
// Menu renders a centred 200 px-wide vertical stack of items inside the
// 240×280 golden viewport. Each row places its label text as a direct
// child of the row's flex container — the deeply-nested-text caveat
// applies, so labels are siblings of the (optional) shortcut and
// submenu-indicator texts, never wrapped in extra containers.

#[test]
fn menu_open() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .item(MenuItem::new("New file"))
                        .item(MenuItem::new("Open\u{2026}"))
                        .item(MenuItem::new("Save")),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_open", img);
}

#[test]
fn menu_hover_item2() {
    // Hit point lands on the second row inside the centred 200 px panel.
    // Panel top = (280 - 3*32) / 2 = 92; row 2 spans y = 124..156.
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .item(MenuItem::new("New file"))
                        .item(MenuItem::new("Open\u{2026}"))
                        .item(MenuItem::new("Save")),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Hover,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_hover_item2", img);
}

#[test]
fn menu_disabled_item() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .item(MenuItem::new("New"))
                        .item(MenuItem::new("Save"))
                        .item(MenuItem::new("Save As\u{2026}").disabled(true)),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_disabled_item", img);
}

#[test]
fn menu_with_separators() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .item(MenuItem::new("Cut"))
                        .item(MenuItem::new("Copy"))
                        .separator()
                        .item(MenuItem::new("Paste")),
                );
            cx.add(on_panel(m, 140.0, 124.0))
        },
        InteractionMode::Default,
        [140, 124],
        (70.0, 62.0),
    );
    check("menu_with_separators", img);
}

#[test]
fn menu_submenu_open() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .item(MenuItem::new("Export"))
                        .item(MenuItem::new("Recent").submenu(true))
                        .item(MenuItem::new("Quit")),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_submenu_open", img);
}

// ---- TOOLTIP -----------------------------------------------------------------
//
// Tooltip wraps the supplied target in a flex_column wrapper and places the
// bubble as an absolutely-positioned child of that wrapper. The outer
// container's `padding_top_px(30)` reserves 30 logical px above the target
// so the bubble at `top = -32` only clips 2 px at the very top — keeping
// the canvas tight enough for the brutal-quality ≥70% coverage rule while
// still proving the bubble renders above the target.

#[test]
fn tooltip_visible() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().padding_y(30.0))
                    .child(tooltip("Save the file").child(button("Save"))),
            )
        },
        InteractionMode::Hover,
        [150, 74],
        (74.0, 50.0),
    );
    check("tooltip_visible", img);
}

#[test]
fn tooltip_offscreen_clamp() {
    let img = render_widget_to_png(
        |cx| cx.add(tooltip("Save the file").clamp_top().child(button("Save"))),
        InteractionMode::Hover,
        [150, 44],
        (74.0, 22.0),
    );
    check("tooltip_offscreen_clamp", img);
}

#[test]
fn tooltip_with_keyboard_shortcut() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().padding_y(30.0))
                    .child(
                        tooltip("Save the file")
                            .shortcut(Some("Ctrl+S"))
                            .child(button("Save")),
                    ),
            )
        },
        InteractionMode::Hover,
        [180, 74],
        (90.0, 50.0),
    );
    check("tooltip_with_keyboard_shortcut", img);
}

// ---- TABLE -------------------------------------------------------------------

#[derive(Clone)]
struct TableRow {
    name: &'static str,
    role: &'static str,
    city: &'static str,
}

fn table_golden_state() -> sigil::widgets::TableState<TableRow> {
    let rows = vec![
        TableRow {
            name: "Ada",
            role: "Engineer",
            city: "London",
        },
        TableRow {
            name: "Grace",
            role: "Admiral",
            city: "Arlington",
        },
        TableRow {
            name: "Edsger",
            role: "Scientist",
            city: "Nuenen",
        },
        TableRow {
            name: "Barbara",
            role: "Architect",
            city: "Cambridge",
        },
    ];
    let text_cell = |value: &'static str| container().child(text(value));
    sigil::widgets::TableState::new(
        rows,
        vec![
            TableColumn::new("name", "Name")
                .sortable(|a: &TableRow, b: &TableRow| a.name.cmp(b.name))
                .width(ColWidth::Fixed(120.0))
                .cell(move |row: &TableRow| text_cell(row.name)),
            TableColumn::new("role", "Role")
                .sortable(|a: &TableRow, b: &TableRow| a.role.cmp(b.role))
                .width(ColWidth::Fixed(96.0))
                .cell(move |row: &TableRow| text_cell(row.role)),
            TableColumn::new("city", "City")
                .width(ColWidth::Fixed(104.0))
                .cell(move |row: &TableRow| text_cell(row.city)),
        ],
    )
}

fn table_golden_view() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
        .class("bg-surface")
        .child(
            table(table_golden_state())
                .row_height(24.0)
                .header_height(28.0)
                .viewport_width(320.0)
                .viewport_height(144.0),
        )
}

#[test]
fn table_default() {
    let img = render_widget_to_png(
        |cx| cx.add(table_golden_view()),
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    check("table_default", img);
}

#[test]
fn table_selected() {
    let img = render_widget_to_png(
        |cx| {
            let state = table_golden_state();
            state.select_only(1);
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
                    .class("bg-surface")
                    .child(
                        table(state)
                            .row_height(24.0)
                            .header_height(28.0)
                            .viewport_width(320.0)
                            .viewport_height(144.0),
                    ),
            )
        },
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    check("table_selected", img);
}

#[test]
fn table_sorted() {
    let img = render_widget_to_png(
        |cx| {
            let state = table_golden_state();
            state.toggle_sort("name");
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
                    .class("bg-surface")
                    .child(
                        table(state)
                            .row_height(24.0)
                            .header_height(28.0)
                            .viewport_width(320.0)
                            .viewport_height(144.0),
                    ),
            )
        },
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    check("table_sorted", img);
}
