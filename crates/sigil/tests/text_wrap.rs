//! `text(...).wrap()` reflows to its containing block; default text hugs.
//!
//! Before this, a text node measured against the ROOT viewport width, so a
//! long label overflowed a narrow container instead of wrapping (found by
//! `layout_invariants`). Wrapping is opt-in so no existing single-line
//! label changes.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

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

/// 7px per character, one 16px line, so height reveals the line count.
struct CharText;
impl TextMeasure for CharText {
    fn measure(&self, content: &TextContent, space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let text = match content {
            TextContent::Static(s) => s.clone(),
            TextContent::Reactive(sig) => sig.peek(),
        };
        let char_w = 7.0_f32;
        #[allow(clippy::cast_precision_loss)]
        let natural = text.chars().count() as f32 * char_w;
        // Honour a fixed width constraint by wrapping into lines, exactly as
        // the real paragraph engine does; ignore it otherwise (hug).
        match space.width {
            sigil_hal::Dim::Fixed(w) if w > 0.0 && natural > w => {
                let per_line = (w / char_w).floor().max(1.0);
                #[allow(clippy::cast_precision_loss)]
                let lines = (text.chars().count() as f32 / per_line).ceil();
                sigil_hal::LogicalSize::new(w, lines * 16.0)
            }
            _ => sigil_hal::LogicalSize::new(natural, 16.0),
        }
    }
}

fn text_height(wrap: bool) -> f32 {
    let mut height = -1.0;
    sigil::reactive::Runtime::scope("text-wrap", || {
        let long = "wrap this considerably longer sentence across several lines";
        let node = if wrap { text(long).wrap() } else { text(long) };
        let mut cx = Cx::new_root();
        // A 120px-wide column inside a WIDE viewport: the sentence is ~413px
        // unwrapped. Non-wrap text measures against the viewport (fits on one
        // line); wrap text measures against the 120px column (reflows).
        let column = container()
            .layout(LayoutStyle::flex_column().fixed_size(120.0, 400.0))
            .child(node);
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(1100.0, 400.0))
                .child(column),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(CharText));
        layout
            .compute(&mut cx, root, Size::fixed(1100.0, 400.0))
            .expect("layout");
        let column_id = *cx.get(root).unwrap().children().first().unwrap();
        let text_id = *cx.get(column_id).unwrap().children().first().unwrap();
        height = layout.result(text_id).unwrap().rect().height;
    });
    height
}

#[test]
fn wrap_reflows_to_multiple_lines() {
    let wrapped = text_height(true);
    assert!(
        wrapped > 16.5,
        "wrapped text in a narrow column should span multiple 16px lines, \
         got {wrapped}",
    );
}

#[test]
fn default_text_stays_a_single_line() {
    let hugged = text_height(false);
    assert!(
        (hugged - 16.0).abs() < 0.5,
        "default (non-wrapping) text must stay one line so existing labels \
         are unchanged, got {hugged}",
    );
}

#[test]
fn wrapped_text_stays_within_its_container_width() {
    sigil::reactive::Runtime::scope("text-wrap-width", || {
        let mut cx = Cx::new_root();
        let column = container()
            .layout(LayoutStyle::flex_column().fixed_size(120.0, 400.0))
            .child(text("a long sentence that must not exceed the column").wrap());
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(1100.0, 400.0))
                .child(column),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(CharText));
        layout
            .compute(&mut cx, root, Size::fixed(1100.0, 400.0))
            .expect("layout");
        let column_id = *cx.get(root).unwrap().children().first().unwrap();
        let text_id = *cx.get(column_id).unwrap().children().first().unwrap();
        let w = layout.result(text_id).unwrap().rect().width;
        assert!(
            w <= 120.5,
            "wrapped text width {w} must fit the 120px column"
        );
    });
}
