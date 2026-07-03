//! Throwaway probe: render menu-bar + code text in Geist on a dark surface at
//! scale 1.0 (the user's native 1x display) so we can inspect real glyph pixels
//! before/after the Slug dilation change. Not a golden — writes to tests/output/.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use app::DefaultTextMeasure;

mod golden_support;
use golden_support::{InteractionMode, render_widget_to_png_with};

use sigil::prelude::*;

const GEIST_SANS: &str =
    "/path/to/fonts/Geist.ttf";
const GEIST_MONO: &str =
    "/path/to/fonts/GeistMono.ttf";

fn install_geist() -> bool {
    let (Ok(sans), Ok(mono)) = (fs::read(GEIST_SANS), fs::read(GEIST_MONO)) else {
        return false;
    };
    sigil::set_default_fonts(sigil::DefaultFonts {
        sans: Some((Arc::new(sans), 0)),
        mono: Some((Arc::new(mono), 0)),
        icon: None,
    });
    true
}

fn probe_root() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().padding(14.0).gap(7.0))
        .class("bg-surface")
        .child(text("File  Edit  Selection  View  Go  Run  Terminal  Help").class("text-xs"))
        .child(text("Explorer   Search   Source Control   Run and Debug").class("text-sm"))
        .child(text("fn render_glyph(atlas: &GlyphAtlas) -> Coverage {").class("text-sm"))
        .child(text("    let coverage = quad_coverage(p0, p1, p2, sample);").class("text-xs"))
        .child(text("the quick brown fox jumps over the lazy dog 0123456789").class("text-sm"))
}

fn render_once(tag: &str) {
    let img = render_widget_to_png_with(
        |cx| cx.add(probe_root().theme(style::Theme::dark())),
        InteractionMode::Default,
        [680, 180],
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    let Some(img) = img else {
        println!("no GPU adapter — skipping text_crisp_probe");
        return;
    };
    let _ = img.save(format!("/tmp/text_probe_{tag}.png"));
    println!("render → /tmp/text_probe_{tag}.png");
}

/// Reproduce the live editor's line pile-up. The shell's code column is
/// `flex_column().grow(1.0)` holding one label per source line. When the file is
/// taller than the pane, Taffy's default flex-shrink (1.0) compresses each row's
/// BOX below its line height — but glyphs still rasterize at full size, so the
/// lines overlap. `grow=false` lets the column take its natural (content) height
/// so rows keep their line height and stack cleanly (the fix is a scroll/clip
/// wrapper or pinning row heights).
fn editor_column(grow: bool) -> NodeBuilder {
    const LINES: &[&str] = &[
        "/// The host shell tree — the host app's native UI.",
        "use std::fs;",
        "use std::path::{Path, PathBuf};",
        "use sigil::prelude::*;",
        "",
        "const MAX_EDITOR_LINES: usize = 200;",
        "fn project_files() -> Vec<(String, PathBuf)> {",
        "    let root = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
        "    let mut out = Vec::new();",
        "    collect_sources(&root, &mut out);",
        "    out.sort();",
        "    out",
        "}",
        "fn work_area() -> NodeBuilder {",
        "    let files = project_files();",
        "    let selected = signal(0_usize);",
        "    container().child(explorer_body(files))",
        "}",
    ];
    // Both branches grow(1.0) like the real editor column. The FIX adds an
    // overflow clip + per-row shrink(0.0); the BUG is bare rows that flex-shrink.
    let column = if grow {
        LayoutStyle::flex_column().grow(1.0).padding(10.0)
    } else {
        LayoutStyle::flex_column()
            .grow(1.0)
            .padding(10.0)
            .overflow_y(Overflow::Hidden)
    };
    let mut code = container().layout(column).class("w-full");
    for line in LINES {
        let text = if line.is_empty() { " " } else { *line };
        let row = label(text.to_owned()).class("text-sm font-mono");
        if grow {
            // `grow=true` == the live (buggy) shell: bare rows in a column that the
            // parent flex-shrinks, compressing every row below its line height.
            code = code.child(row);
        } else {
            // `grow=false` == the fix: each row pinned with shrink(0.0) so flex can't
            // compress it below its measured box; the column clips the overflow.
            code = code.child(
                container()
                    .layout(LayoutStyle::flex_row().shrink(0.0))
                    .child(row),
            );
        }
    }
    container()
        .theme(style::Theme::dark())
        .layout(LayoutStyle::flex_column().fill())
        .class("bg-surface")
        .child(code)
}

/// Does `overflow_y(Hidden)` actually clip PIXELS (not just layout)? A 70px box
/// holding 8 tall rows, with a sentinel label BELOW it. If clipping works, only
/// ~4 rows show and the sentinel is clean; if the renderer ignores overflow, all
/// 8 rows spill over the sentinel.
fn clip_probe() -> NodeBuilder {
    let mut box_ = container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(260.0, 70.0)
                .overflow_y(Overflow::Hidden),
        )
        .class("bg-surface-hover");
    for i in 0..8 {
        // shrink(0) so the rows genuinely OVERFLOW the 70px box (not compress) — that
        // is what overflow_y(Hidden) must clip. ~4 rows fit; rows 4..8 must be cut.
        box_ = box_.child(
            container()
                .layout(LayoutStyle::flex_row().shrink(0.0))
                .child(label(format!("clipped line {i}")).class("text-sm font-mono")),
        );
    }
    container()
        .theme(style::Theme::dark())
        .layout(LayoutStyle::flex_column().fill().gap(8.0).padding(8.0))
        .class("bg-surface")
        .child(box_)
        .child(label("=== SENTINEL BELOW THE BOX ===").class("text-sm"))
}

#[test]
fn render_clip_probe() {
    if !install_geist() {
        println!("Geist not present — skipping clip_probe");
        return;
    }
    if let Some(img) = render_widget_to_png_with(
        |cx| cx.add(clip_probe()),
        InteractionMode::Default,
        [300, 200],
        (0.0, 0.0),
        Box::new(DefaultTextMeasure::new()),
    ) {
        let _ = img.save("/tmp/clip_probe.png");
    }
    println!("clip probe → /tmp/clip_probe.png");
}

/// The editor's real shape: a tall monospace column wrapped in `scroll()`. At
/// rest it shows the file top, clipped to the viewport, with a scrollbar thumb on
/// the right. Proves overflow now scissors AND the scroll widget composes.
fn editor_scroll_root() -> NodeBuilder {
    let mut code = container().layout(LayoutStyle::flex_column().padding(8.0));
    for i in 0..40 {
        code = code.child(
            container()
                .layout(LayoutStyle::flex_row().shrink(0.0))
                .child(
                    label(format!("{i:>3}  fn line_{i}() {{ /* source */ }}"))
                        .class("text-sm font-mono"),
                ),
        );
    }
    let extent = 40.0_f32.mul_add(16.8, 16.0);
    container()
        .theme(style::Theme::dark())
        .layout(LayoutStyle::flex_column().fill())
        .class("bg-surface")
        .child(scroll(code, extent).width(440.0).height(170.0))
}

/// Reproduce + fix the long-line overlap: a mix of short code lines and LONG
/// comment lines (wider than the viewport). mode 0 = current shell (rows that
/// wrap → measure/render height mismatch → overlap). mode 1 = fixed-height rows
/// that clip their own overflow (uniform single-line rows, long tail cut).
fn editor_wrap_root(mode: u8) -> NodeBuilder {
    // 24 lines — far more than the 120px viewport (~7) — to test whether off-screen
    // lines are CULLED or spill over the sentinel below the scroll.
    let mut lines: Vec<String> = Vec::new();
    for i in 0..24 {
        if i % 4 == 0 {
            lines.push(format!(
                "/// {i:>2} a deliberately very long doc comment line that runs well past the editor viewport width to exercise wrapping and horizontal clip behaviour"
            ));
        } else {
            lines.push(format!("{i:>2}  fn line_{i}() {{ let x = {i}; }}"));
        }
    }
    // mode 0 = one node PER LINE, each row WIDE (no wrap) → per-line runs the clip
    //          culls when off-screen.  mode 1 = ONE text node (one glyph run) for
    //          the whole file → can the per-run clip still cull off-screen lines?
    let code = if mode == 0 {
        // Per-line labels placed DIRECTLY in a wide column (no flex_row wrapper):
        // each label is its own glyph run (off-screen lines cull) AND inherits the
        // wide column width, so it never wraps (measure == render → no overlap).
        let mut col = container().layout(LayoutStyle::flex_column().padding(8.0).width(1400.0));
        for line in &lines {
            col = col.child(label(line.clone()).class("text-sm font-mono"));
        }
        col
    } else {
        container()
            .layout(LayoutStyle::flex_column().padding(8.0).width(1400.0))
            .child(text(lines.join("\n")).class("text-sm font-mono"))
    };
    let extent = 24.0_f32.mul_add(16.8, 16.0);
    container()
        .theme(style::Theme::dark())
        .layout(LayoutStyle::flex_column().fill().gap(6.0))
        .class("bg-surface")
        .child(scroll(code, extent).width(440.0).height(120.0))
        .child(label("=== SENTINEL — must stay clean ===").class("text-sm"))
}

#[test]
fn render_editor_wrap() {
    if !install_geist() {
        println!("Geist not present — skipping editor_wrap");
        return;
    }
    for mode in [0_u8, 1] {
        if let Some(img) = render_widget_to_png_with(
            move |cx| cx.add(editor_wrap_root(mode)),
            InteractionMode::Default,
            [460, 200],
            (0.0, 0.0),
            Box::new(DefaultTextMeasure::new()),
        ) {
            let tag = if mode == 0 { "bug" } else { "fix" };
            let _ = img.save(format!("/tmp/editor_wrap_{tag}.png"));
        }
    }
    println!("editor wrap → /tmp/editor_wrap_bug.png /tmp/editor_wrap_fix.png");
}

#[test]
fn render_editor_scroll() {
    if !install_geist() {
        println!("Geist not present — skipping editor_scroll");
        return;
    }
    if let Some(img) = render_widget_to_png_with(
        |cx| cx.add(editor_scroll_root()),
        InteractionMode::Default,
        [460, 190],
        (0.0, 0.0),
        Box::new(DefaultTextMeasure::new()),
    ) {
        let _ = img.save("/tmp/editor_scroll.png");
    }
    println!("editor scroll → /tmp/editor_scroll.png");
}

#[test]
fn render_editor_overlap() {
    if !install_geist() {
        println!("Geist not present — skipping editor_overlap");
        return;
    }
    for (tag, grow) in [("overlap", true), ("fixed", false)] {
        if let Some(img) = render_widget_to_png_with(
            move |cx| cx.add(editor_column(grow)),
            InteractionMode::Default,
            [560, 200],
            (0.0, 0.0),
            Box::new(DefaultTextMeasure::new()),
        ) {
            let _ = img.save(format!("/tmp/editor_{tag}.png"));
        }
    }
    println!("editor probe → /tmp/editor_overlap.png /tmp/editor_fixed.png");
}

/// Two clickable rows over a `children_reactive` content area — the exact pattern the
/// console-ui shell uses (pointer_down → selection signal → reactive content → compute flush).
fn selection_probe_root(files: Vec<(&'static str, &'static str)>) -> NodeBuilder {
    let sel = signal(0_usize);
    let mut rows = container().layout(LayoutStyle::flex_column().gap(4.0).padding(8.0));
    for (idx, (name, _)) in files.iter().enumerate() {
        let name = (*name).to_owned();
        rows = rows.child(
            container()
                .role(Role::Button)
                .focusable(true)
                .layout(
                    LayoutStyle::flex_row()
                        .height(24.0)
                        .padding_x(6.0)
                        .align_items(AlignItems::Center),
                )
                .class("w-full")
                .pointer_down(move |ev| {
                    sel.set(idx);
                    ev.stop_propagation();
                })
                .child(label(name).class("text-sm")),
        );
    }
    let bodies: Vec<String> = files.iter().map(|(_, body)| (*body).to_owned()).collect();
    let content = container()
        .layout(LayoutStyle::flex_column().grow(1.0).padding(8.0))
        .class("w-full")
        .children_reactive(move || {
            let body = bodies.get(sel.get()).cloned().unwrap_or_default();
            body.lines()
                .map(|line| label(line.to_owned()).class("text-sm font-mono"))
                .collect::<Vec<_>>()
        });
    container()
        .theme(style::Theme::dark())
        .layout(LayoutStyle::flex_column().fill())
        .class("bg-surface")
        .child(rows)
        .child(content)
}

#[test]
fn render_selection_probe() {
    if !install_geist() {
        println!("Geist fonts not present — skipping selection_probe");
        return;
    }
    let files = vec![
        (
            "alpha.rs",
            "fn alpha() -> u32 {\n    let answer = 42;\n    answer\n}",
        ),
        (
            "beta.rs",
            "struct Beta;\nimpl Beta {\n    fn run(&self) {}\n}",
        ),
    ];
    // Default selection (row 0 → alpha.rs).
    if let Some(img) = render_widget_to_png_with(
        move |cx| cx.add(selection_probe_root(files.clone())),
        InteractionMode::Default,
        [420, 200],
        (0.0, 0.0),
        Box::new(DefaultTextMeasure::new()),
    ) {
        let _ = img.save("/tmp/sel_default.png");
    }
    // Press row 1 (beta.rs) — center of the second row at y ≈ 8 + 24 + 4 + 12 = 48.
    let files2 = vec![
        (
            "alpha.rs",
            "fn alpha() -> u32 {\n    let answer = 42;\n    answer\n}",
        ),
        (
            "beta.rs",
            "struct Beta;\nimpl Beta {\n    fn run(&self) {}\n}",
        ),
    ];
    if let Some(img) = render_widget_to_png_with(
        move |cx| cx.add(selection_probe_root(files2.clone())),
        InteractionMode::Pressed,
        [420, 200],
        (40.0, 48.0),
        Box::new(DefaultTextMeasure::new()),
    ) {
        let _ = img.save("/tmp/sel_pressed.png");
    }
    println!("selection probe → /tmp/sel_default.png /tmp/sel_pressed.png");
}

/// A/B Geist vs San Francisco through the SAME (thread-local) paint engine: render Geist,
/// flip the process-global fonts to the macOS system SF, render again. If the second image
/// shows SF glyphs, `set_default_fonts` + `OwnedTextEngine::refresh_fonts` switched a live
/// engine at runtime (and SF loaded + parsed from disk).
#[test]
fn render_font_ab() {
    if !install_geist() {
        println!("Geist not present — skipping font_ab");
        return;
    }
    render_once("font_geist");

    let (Ok(sans), Ok(mono)) = (
        fs::read("/System/Library/Fonts/SFNS.ttf"),
        fs::read("/System/Library/Fonts/SFNSMono.ttf"),
    ) else {
        println!("San Francisco not present — skipping SF half");
        return;
    };
    sigil::set_default_fonts(sigil::DefaultFonts {
        sans: Some((Arc::new(sans), 0)),
        mono: Some((Arc::new(mono), 0)),
        icon: None,
    });
    render_once("font_sf");
    install_geist(); // restore for any later test on this thread
}

#[test]
fn render_text_crisp_probe() {
    if !install_geist() {
        println!("Geist fonts not present — skipping text_crisp_probe");
        return;
    }
    // Render both A/B modes so we can confirm the runtime toggle changes real pixels.
    sigil::set_text_tuning(sigil::TextTuning::default());
    render_once("tuned");
    sigil::set_text_tuning(sigil::TextTuning {
        enabled: false,
        ..sigil::TextTuning::default()
    });
    render_once("raw");
    sigil::set_text_tuning(sigil::TextTuning::default());
}
