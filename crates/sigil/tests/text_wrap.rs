//! Default `text()` wraps to a definite-width box. `.nowrap()` hugs one line.

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;

/// 7px per character, one 16px line, so height reveals the line count.
struct CharText;
impl TextMeasure for CharText {
    fn measure(&self, content: &TextContent, space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let text = match content {
            TextContent::Static(s) => s.clone(),
            TextContent::Reactive(sig) => sig.peek(),
            TextContent::Compute(_) => content.resolve(),
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

/// 100 chars = 700px at 7px/char — longer than the 400px viewport below.
/// Gap 24(a): a `wrap = false` run measured against the definite viewport
/// width wrapped at the viewport edge, so an ellipsized cell reported two
/// lines and bled over the next row. Measurement must be unconstrained
/// (max-content) regardless of the node's layout style.
const LONGER_THAN_VIEWPORT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn row_cell_rect(style: LayoutStyle) -> sigil_hal::Rect {
    let mut rect = sigil_hal::Rect::new(sigil_hal::Point::new(0.0, 0.0), -1.0, -1.0);
    sigil::reactive::Runtime::scope("text-wrap-off-viewport", || {
        let mut cx = Cx::new_root();
        let cell = label(LONGER_THAN_VIEWPORT).ellipsize().layout(style);
        let root = cx.add(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(400.0, 28.0)
                        .align_items(AlignItems::Start),
                )
                .child(cell),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(CharText));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 28.0))
            .expect("layout");
        let text_id = *cx.get(root).unwrap().children().first().unwrap();
        rect = layout.result(text_id).unwrap().rect();
    });
    rect
}

#[test]
fn ellipsize_longer_than_viewport_stays_one_line_flex_row() {
    let rect = row_cell_rect(LayoutStyle::flex_row().width(96.0));
    assert!(
        (rect.height - 16.0).abs() < 0.5,
        "ellipsize text in a flex_row box must stay one line, got height {}",
        rect.height,
    );
    assert!(
        (rect.width - 96.0).abs() < 0.5,
        "ellipsize text keeps its 96px box, got width {}",
        rect.width,
    );
}

#[test]
fn ellipsize_longer_than_viewport_stays_one_line_block() {
    let rect = row_cell_rect(LayoutStyle::block().width(96.0));
    assert!(
        (rect.height - 16.0).abs() < 0.5,
        "ellipsize text in a block box must stay one line, got height {}",
        rect.height,
    );
}

#[test]
fn ellipsize_longer_than_viewport_stays_one_line_when_grown() {
    let rect = row_cell_rect(LayoutStyle::block().grow(1.0));
    assert!(
        (rect.height - 16.0).abs() < 0.5,
        "ellipsize text with grow(1.0) must stay one line, got height {}",
        rect.height,
    );
}

#[test]
fn ellipsize_leaf_shrinks_instead_of_crushing_fixed_siblings() {
    // A 200px row: a fixed 96px cell plus an ellipsized cell whose content
    // wants 420px. When the ellipsized cell is floored at its content
    // width (CSS §4.5 automatic minimum), it cannot shrink and the fixed
    // cell is crushed to zero. An ellipsizing leaf truncates instead, so
    // its automatic minimum is zero and both cells keep a share.
    sigil::reactive::Runtime::scope("text-ellipsize-floor", || {
        let mut cx = Cx::new_root();
        let fixed = container().layout(LayoutStyle::block().width(96.0).height(16.0));
        let flex = label(LONG).ellipsize().layout(LayoutStyle::block());
        let root = cx.add(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(200.0, 28.0)
                        .align_items(AlignItems::Start),
                )
                .child(fixed)
                .child(flex),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(CharText));
        layout
            .compute(&mut cx, root, Size::fixed(200.0, 28.0))
            .expect("layout");
        let row = cx.get(root).unwrap().children().to_vec();
        let fixed_w = layout.result(row[0]).unwrap().rect().width;
        let flex_rect = layout.result(row[1]).unwrap().rect();
        assert!(
            fixed_w > 60.0,
            "fixed sibling must keep most of its 96px, got {fixed_w}",
        );
        assert!(
            (flex_rect.height - 16.0).abs() < 0.5,
            "ellipsized cell stays one line, got height {}",
            flex_rect.height,
        );
    });
}
