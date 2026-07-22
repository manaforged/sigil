//! Structural layout invariants over REAL compositions at MANY widths.
//!
//! Why this file exists: on 2026-07-20 the suite was 1672/1672 green while
//! the widget playground rendered with every column collapsed onto the left
//! margin, and separately with a dropdown whose label had vanished. Neither
//! was visible to the suite, because:
//!
//!   * `widgets.rs` runs all 31 of its cases at one 320x120 viewport, so
//!     nothing exercises adaptation;
//!   * its `StubMeasure` returns 40x16 for EVERY string, so text extent
//!     bugs cannot be observed;
//!   * the goldens are single widgets on a fixed canvas, so composition
//!     bugs are out of frame — and regenerating them enshrines whatever is
//!     currently wrong.
//!
//! These checks are deliberately mechanical rather than pixel-based: they
//! assert properties that must hold for ANY correct layout, so they keep
//! working as the design changes. A golden says "it looks like this"; these
//! say "it is not broken".

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used, clippy::panic)]

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
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;

/// Text measure proportional to the CONTENT, unlike the fixed 40x16 stub
/// the widget suite uses. A layout that only holds together because every
/// string measures the same is not actually holding together.
struct ProportionalText;

impl TextMeasure for ProportionalText {
    fn measure(&self, content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let len = match content {
            TextContent::Static(s) => s.len(),
            TextContent::Reactive(sig) => sig.peek().len(),
        };
        #[allow(clippy::cast_precision_loss)]
        sigil_hal::LogicalSize::new(len as f32 * 7.0, 16.0)
    }
}

#[derive(Debug)]
struct Violation {
    depth: usize,
    what: String,
}

const EPS: f32 = 0.5;

/// Walk the laid-out tree and collect every structural violation.
fn violations(cx: &Cx, layout: &LayoutEngine, root: NodeId) -> Vec<Violation> {
    let mut out = Vec::new();
    visit(cx, layout, root, (0, false), &mut out);
    out
}

/// `clipped` is true once ANY ancestor clips: overflow inside a clipped
/// subtree is contained by that ancestor, so it is not a violation. A
/// `text_field`'s scrolling track is deliberately wider than the field and is
/// held in by the field's `overflow: hidden` viewport, not by its own box.
/// `(depth, clipped)` — how deep we are, and whether an ancestor clips.
type WalkState = (usize, bool);

fn visit(cx: &Cx, layout: &LayoutEngine, id: NodeId, state: WalkState, out: &mut Vec<Violation>) {
    let (depth, clipped) = state;
    let Some(node) = cx.get(id) else { return };
    let children: Vec<NodeId> = node.children().to_vec();
    let Some(result) = layout.result(id) else {
        return;
    };
    let rect = result.rect();
    // A container that clips is ENTITLED to oversized children — that is
    // what `overflow: hidden` means. Only non-clipping parents are checked,
    // so this does not fire on deliberately-clipped text.
    let clips = result.clips();

    // A node that HAS children but resolved to zero on an axis has
    // collapsed: its contents cannot possibly be visible. This is the exact
    // shape of the flex-grow and block-dispatch bugs.
    if !children.is_empty() && (rect.width <= EPS || rect.height <= EPS) {
        out.push(Violation {
            depth,
            what: format!(
                "collapsed: {}x{} but has {} children",
                rect.width,
                rect.height,
                children.len()
            ),
        });
    }

    for child in &children {
        let Some(c) = layout.result(*child).map(|r| r.rect()) else {
            continue;
        };
        // Children must stay inside their parent's box. Overflow is how the
        // sliver text_field spilled its text outside the field.
        let overflows_right = c.origin.x + c.width > rect.origin.x + rect.width + EPS;
        let overflows_bottom = c.origin.y + c.height > rect.origin.y + rect.height + EPS;
        if !clips && !clipped && (overflows_right || overflows_bottom) {
            out.push(Violation {
                depth,
                what: format!(
                    "child overflows parent: child {}x{} at ({},{}) vs parent {}x{} at ({},{})",
                    c.width,
                    c.height,
                    c.origin.x,
                    c.origin.y,
                    rect.width,
                    rect.height,
                    rect.origin.x,
                    rect.origin.y
                ),
            });
        }
    }

    for child in children {
        visit(cx, layout, child, (depth + 1, clipped || clips), out);
    }
}

/// Lay `build` out at `width` and assert it has no structural violations.
fn assert_sound(label: &str, width: f32, build: impl FnOnce() -> NodeBuilder) {
    sigil::reactive::Runtime::scope("layout-invariants", || {
        let mut cx = Cx::new_root();
        let root = cx.add(build());
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalText));
        layout
            .compute(&mut cx, root, Size::fixed(width, 2000.0))
            .expect("layout computes");
        let found = violations(&cx, &layout, root);
        assert!(
            found.is_empty(),
            "{label} at width {width}: {} structural violation(s)\n{}",
            found.len(),
            found
                .iter()
                .take(8)
                .map(|v| format!("  [depth {}] {}", v.depth, v.what))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    });
}

/// Widths a real app actually hits: a narrow pane, a normal window, wide.
const WIDTHS: [f32; 4] = [320.0, 640.0, 1100.0, 1600.0];

fn labeled_field(label: &'static str, control: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(4.0))
        .child(text(label))
        .child(control)
}

/// The `flex: 1 1 0` equal-columns idiom, which is how real form layouts
/// are built and how every widget ends up sharing a row.
fn columns(children: Vec<NodeBuilder>) -> NodeBuilder {
    let mut row = container().layout(LayoutStyle::flex_row().fill_width().gap(12.0));
    for child in children {
        row = row.child(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0).width(0.0))
                .child(child),
        );
    }
    row
}

#[test]
fn form_row_of_widgets_is_sound_at_every_width() {
    for width in WIDTHS {
        assert_sound("form row", width, || {
            columns(vec![
                labeled_field("text_field", text_field(signal(String::from("hello")))),
                labeled_field("dropdown", dropdown(signal(0_usize), vec!["Banana".into()])),
                labeled_field("slider", slider(signal(0.5_f32), 0.0..=1.0)),
            ])
        });
    }
}

#[test]
fn stacked_widgets_are_sound_at_every_width() {
    for width in WIDTHS {
        assert_sound("stacked column", width, || {
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fill_width()
                        .gap(8.0)
                        .padding(16.0),
                )
                .child(labeled_field("progress", progress(signal(0.5_f32), 1.0)))
                .child(labeled_field("toggle", toggle(signal(true))))
                .child(labeled_field("button", button("Save")))
                .child(labeled_field(
                    "checkbox",
                    checkbox(signal(true)).label("enabled"),
                ))
        });
    }
}

/// Long CONTENT inside widgets must stay inside them — the widgets clip it.
///
/// A content-blind text stub (the widget suite's returns 40x16 for every
/// string) cannot see this class of bug at all.
///
/// Scope note: a bare `text()` node hugs its content by default and can
/// still overflow a narrow container. Opt into reflow with `text(..).wrap()`
/// (see `text_wrap.rs`). Single-line ellipsis for the non-wrapping case is
/// the remaining gap.
#[test]
fn long_content_stays_inside_its_widget() {
    for width in WIDTHS {
        assert_sound("long content", width, || {
            columns(vec![
                labeled_field(
                    "field",
                    text_field(signal(String::from(
                        "a rather long value sitting inside the field, well past its width",
                    ))),
                ),
                labeled_field(
                    "menu",
                    dropdown(
                        signal(0_usize),
                        vec!["an option label considerably wider than the control".into()],
                    ),
                ),
            ])
        });
    }
}

/// Content inside a `scroll` viewport must survive at every width.
///
/// This shape is what the block-dispatch bug broke: the viewport's content
/// wrapper is a `Display::Block` flex item that grows into the space the
/// scrollbar gutter leaves, and `BlockSolver` re-derived its width from its
/// own style, collapsing it (and therefore the whole page) to zero. Without
/// a scroll in the fixture, the checker cannot see that class at all.
#[test]
fn scrolled_content_is_sound_at_every_width() {
    for width in WIDTHS {
        assert_sound("scrolled page", width, || {
            let page = container()
                .layout(LayoutStyle::flex_column().fill_width().gap(8.0))
                .child(labeled_field(
                    "field",
                    text_field(signal(String::from("hi"))),
                ))
                .child(labeled_field("bar", progress(signal(0.5_f32), 1.0)))
                .child(columns(vec![
                    labeled_field("left", progress(signal(0.25_f32), 1.0)),
                    labeled_field("right", progress(signal(0.75_f32), 1.0)),
                ]));
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(sigil::widgets::scroll(page, 600.0_f32))
        });
    }
}

/// A container whose width changes between passes must re-resolve. Kimi's
/// review noted no test in the repo did this.
#[test]
fn layout_re_resolves_when_the_container_resizes() {
    sigil::reactive::Runtime::scope("layout-invariants-resize", || {
        let mut cx = Cx::new_root();
        // Filling widgets, deliberately: a FIXED-size control (button keeps
        // its themed `control_width`) legitimately overflows a cell narrower
        // than itself, which would make this about button sizing rather than
        // about re-resolution.
        let root = cx.add(columns(vec![
            labeled_field("a", progress(signal(0.5_f32), 1.0)),
            labeled_field("b", progress(signal(0.5_f32), 1.0)),
        ]));
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalText));

        let mut widths = Vec::new();
        for w in [400.0_f32, 900.0, 250.0] {
            layout
                .compute(&mut cx, root, Size::fixed(w, 600.0))
                .expect("layout computes");
            let found = violations(&cx, &layout, root);
            assert!(
                found.is_empty(),
                "structural violation after resizing to {w}:\n{}",
                found
                    .iter()
                    .map(|v| format!("  [depth {}] {}", v.depth, v.what))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            let first = *cx.get(root).unwrap().children().first().unwrap();
            widths.push(layout.result(first).unwrap().rect().width);
        }

        // Each cell is half the row minus the gap; the cell must TRACK the
        // container, not keep a stale width from the previous pass.
        let expected = [194.0_f32, 444.0, 119.0];
        for (got, want) in widths.iter().zip(expected) {
            assert!(
                (got - want).abs() < 1.0,
                "cell should track the container across resizes: got {widths:?}, \
                 expected ~{expected:?}",
            );
        }
    });
}
