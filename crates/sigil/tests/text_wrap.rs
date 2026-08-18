//! Default `text()` wraps to a definite-width box. `.nowrap()` hugs one line.

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
use sigil_profile as _;
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
            TextContent::Compute(build) => build(),
        };
        let char_w = 7.0_f32;
        #[allow(clippy::cast_precision_loss)]
        let natural = text.chars().count() as f32 * char_w;
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

fn text_height(node: TextBuilder) -> f32 {
    let mut height = -1.0;
    sigil::reactive::Runtime::scope("text-wrap", || {
        let long = node;
        let mut cx = Cx::new_root();
        let column = container()
            .layout(LayoutStyle::flex_column().fixed_size(120.0, 400.0))
            .child(long);
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

const LONG: &str = "wrap this considerably longer sentence across several lines";

#[test]
fn default_text_wraps_in_a_narrow_column() {
    let wrapped = text_height(text(LONG));
    assert!(
        wrapped > 16.5,
        "default text in a 120px column must wrap, got {wrapped}",
    );
}

#[test]
fn label_stays_a_single_line() {
    let hugged = text_height(label(LONG));
    assert!(
        (hugged - 16.0).abs() < 0.5,
        "label is chrome and must stay one line, got {hugged}",
    );
}

#[test]
fn nowrap_stays_a_single_line() {
    let hugged = text_height(text(LONG).nowrap());
    assert!(
        (hugged - 16.0).abs() < 0.5,
        "nowrap text must stay one line, got {hugged}",
    );
}

#[test]
fn wrapped_text_stays_within_its_container_width() {
    sigil::reactive::Runtime::scope("text-wrap-width", || {
        let mut cx = Cx::new_root();
        let column = container()
            .layout(LayoutStyle::flex_column().fixed_size(120.0, 400.0))
            .child(text("a long sentence that must not exceed the column"));
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
            "default text width {w} must fit the 120px column"
        );
    });
}
