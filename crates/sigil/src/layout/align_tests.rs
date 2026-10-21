use crate::geometry::{LogicalSize, Size};
use crate::kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use crate::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use crate::reactive::Runtime;
use crate::widgets::Axis;

use super::{Misalignment, Tolerance, misalignments};

struct NoText;

impl TextMeasure for NoText {
    fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

fn cell(width: f32) -> NodeBuilder {
    container().layout(LayoutStyle::block().fixed_size(width, 2.0))
}

fn spacer() -> NodeBuilder {
    container().layout(LayoutStyle::block().fixed_size(0.0, 2.0).grow(1.0))
}

fn with(parent: NodeBuilder, children: Vec<NodeBuilder>) -> NodeBuilder {
    children.into_iter().fold(parent, NodeBuilder::child)
}

fn row(cells: Vec<NodeBuilder>) -> NodeBuilder {
    with(
        container().layout(LayoutStyle::flex_row().fixed_size(200.0, 2.0)),
        cells,
    )
}

fn stack(rows: Vec<NodeBuilder>) -> NodeBuilder {
    with(
        container().layout(LayoutStyle::flex_column().fixed_size(200.0, 100.0)),
        rows,
    )
}

fn found(name: &'static str, tree: NodeBuilder) -> Vec<Misalignment> {
    Runtime::scope(name, || {
        let mut cx = Cx::new_root();
        let root: NodeId = cx.add(tree);
        let mut engine = LayoutEngine::with_text_measure(Box::new(NoText));
        engine
            .compute(&mut cx, root, Size::fixed(200.0, 100.0))
            .expect("layout");
        misalignments(&cx, &engine, root, Tolerance::points())
    })
}

fn ragged() -> Vec<NodeBuilder> {
    [44.0, 52.0, 58.0]
        .into_iter()
        .map(|title| row(vec![cell(title), cell(10.0), cell(20.0)]))
        .collect()
}

#[test]
fn ragged_middle_column_is_reported() {
    let found = found("align-ragged", stack(ragged()));
    let columns: Vec<usize> = found
        .iter()
        .filter_map(|item| match item {
            Misalignment::Column { column, .. } => Some(*column),
            Misalignment::Inset { .. } => None,
        })
        .collect();
    assert_eq!(columns, vec![1, 2], "{found:?}");
    let text = found.first().map(ToString::to_string).unwrap_or_default();
    assert!(text.contains("column 1 of <Container"), "{text}");
    assert!(text.contains("[44.0, 52.0, 58.0]"), "{text}");
}

#[test]
fn fixed_width_columns_are_clean() {
    let rows = (0..3)
        .map(|_| row(vec![cell(60.0), cell(10.0), cell(20.0)]))
        .collect();
    assert_eq!(found("align-fixed", stack(rows)), Vec::new());
}

#[test]
fn right_aligned_numbers_are_clean() {
    let rows = [12.0, 30.0, 6.0]
        .into_iter()
        .map(|number| row(vec![spacer(), cell(number)]))
        .collect();
    assert_eq!(found("align-right", stack(rows)), Vec::new());
}

#[test]
fn asymmetric_container_is_skipped() {
    let tree = stack(ragged()).asymmetric("titles run ragged on purpose");
    assert_eq!(found("align-waived", tree), Vec::new());
}

fn inset(left: f32, right: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(100.0, 10.0)
                .padding_y(3.0)
                .padding_left(left)
                .padding_right(right),
        )
        .child(container().layout(LayoutStyle::block().height(4.0)))
}

#[test]
fn near_miss_inset_is_reported_and_wide_one_is_not() {
    let near = found("align-inset-near", inset(4.0, 2.0));
    assert!(
        matches!(
            near.as_slice(),
            [Misalignment::Inset { axis: Axis::X, start, end, .. }]
                if (*start - 4.0).abs() < f32::EPSILON && (*end - 2.0).abs() < f32::EPSILON
        ),
        "{near:?}"
    );
    assert_eq!(found("align-inset-wide", inset(42.0, 2.0)), Vec::new());
}

#[test]
fn a_nested_near_miss_inset_is_measured_from_its_own_parent() {
    let tree = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(200.0, 60.0)
                .padding_left(30.0)
                .padding_y(20.0),
        )
        .child(inset(4.0, 2.0));
    let near = found("align-inset-nested", tree);
    assert!(
        near.iter().any(|item| matches!(
            item,
            Misalignment::Inset { axis: Axis::X, start, end, .. }
                if (*start - 4.0).abs() < f32::EPSILON && (*end - 2.0).abs() < f32::EPSILON
        )),
        "{near:?}"
    );
}

#[test]
fn a_header_row_does_not_hide_ragged_rows_below_it() {
    let mut rows = vec![row(vec![cell(8.0), cell(30.0)])];
    rows.extend(ragged());
    let columns: Vec<usize> = found("align-header", stack(rows))
        .iter()
        .filter_map(|item| match item {
            Misalignment::Column { column, .. } => Some(*column),
            Misalignment::Inset { .. } => None,
        })
        .collect();
    assert_eq!(columns, vec![1, 2]);
}

#[test]
fn a_footer_styled_unlike_the_rows_is_not_compared_with_them() {
    let footer = with(
        container().layout(LayoutStyle::flex_row().fixed_size(200.0, 4.0)),
        vec![cell(90.0), cell(10.0), cell(20.0)],
    );
    let rows = (0..2)
        .map(|_| row(vec![cell(60.0), cell(10.0), cell(20.0)]))
        .chain([footer])
        .collect();
    assert_eq!(found("align-footer", stack(rows)), Vec::new());
}

#[test]
fn text_inside_a_wider_box_is_not_an_inset() {
    let tree = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(10.0, 10.0)
                .padding_left(1.0),
        )
        .child(crate::kernel::text("label").layout(LayoutStyle::block().width(5.0)));
    assert_eq!(found("align-text", tree), Vec::new());
}
