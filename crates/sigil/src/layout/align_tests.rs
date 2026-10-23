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
    found_at(name, tree, Tolerance::points())
}

fn found_at(name: &'static str, tree: NodeBuilder, tolerance: Tolerance) -> Vec<Misalignment> {
    Runtime::scope(name, || {
        let mut cx = Cx::new_root();
        let root: NodeId = cx.add(tree);
        let mut engine = LayoutEngine::with_text_measure(Box::new(NoText));
        engine
            .compute(&mut cx, root, Size::fixed(200.0, 100.0))
            .expect("layout");
        misalignments(&cx, &engine, root, tolerance)
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

#[test]
fn centered_columns_are_clean() {
    let rows = vec![
        row(vec![cell(8.0), cell(44.0)]),
        row(vec![cell(20.0), cell(20.0)]),
    ];
    assert_eq!(found("align-centered", stack(rows)), Vec::new());
}

#[test]
fn a_vertical_near_miss_is_measured_from_a_parent_below_the_top() {
    let uneven = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(100.0, 10.0)
                .padding_x(3.0)
                .padding_top(4.0)
                .padding_bottom(2.0),
        )
        .child(container().layout(LayoutStyle::block().height(4.0)));
    let tree = container()
        .layout(
            LayoutStyle::block()
                .fixed_size(200.0, 60.0)
                .padding_top(2.0),
        )
        .child(uneven);
    let near = found("align-inset-vertical", tree);
    assert!(
        matches!(
            near.as_slice(),
            [Misalignment::Inset { axis: Axis::Y, start, end, .. }]
                if (*start - 4.0).abs() < f32::EPSILON && (*end - 2.0).abs() < f32::EPSILON
        ),
        "{near:?}"
    );
}

#[test]
fn a_cell_host_reports_insets_up_to_three_cells_apart() {
    let cell = crate::Host::CELL;
    let near = found_at(
        "align-cells-near",
        inset(3.0 * cell, cell),
        Tolerance::cells(),
    );
    assert!(
        matches!(near.as_slice(), [Misalignment::Inset { axis: Axis::X, .. }]),
        "{near:?}"
    );
    let wide = found_at(
        "align-cells-wide",
        inset(5.0 * cell, cell),
        Tolerance::cells(),
    );
    assert_eq!(wide, Vec::new());
}

fn grid(widths: &[f32], rows: usize, shift: Option<(usize, usize, f32)>) -> NodeBuilder {
    stack(
        (0..rows)
            .map(|at| {
                row(widths
                    .iter()
                    .enumerate()
                    .map(|(column, width)| {
                        let extra = shift
                            .filter(|(row, shifted, _)| *row == at && *shifted == column)
                            .map_or(0.0, |(_, _, delta)| delta);
                        cell(width + extra)
                    })
                    .collect())
            })
            .collect(),
    )
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]

    #[test]
    fn one_shifted_cell_in_a_grid_of_shared_widths_is_reported(
        widths in proptest::collection::vec(4_u8..30, 3..6),
        rows in 2_usize..6,
        row_at in proptest::prelude::any::<proptest::sample::Index>(),
        column_at in proptest::prelude::any::<proptest::sample::Index>(),
        delta in 1_u8..6,
    ) {
        let widths: Vec<f32> = widths.into_iter().map(f32::from).collect();
        proptest::prop_assert_eq!(found("align-grid", grid(&widths, rows, None)), Vec::new());
        let column = column_at.index(widths.len() - 1);
        let shift = (row_at.index(rows), column, f32::from(delta));
        let caught = found("align-grid", grid(&widths, rows, Some(shift)));
        proptest::prop_assert!(
            caught.iter().any(|item| matches!(
                item,
                Misalignment::Column { column: next, .. } if *next == column + 1
            )),
            "{caught:?}"
        );
    }

    #[test]
    fn an_even_inset_is_clean_and_an_uneven_one_is_reported(pad in 1_u8..20, delta in 1_u8..5) {
        let pad = f32::from(pad);
        proptest::prop_assert_eq!(found("align-pad", inset(pad, pad)), Vec::new());
        let near = found("align-pad", inset(pad + f32::from(delta), pad));
        proptest::prop_assert!(
            matches!(near.as_slice(), [Misalignment::Inset { axis: Axis::X, .. }]),
            "{near:?}"
        );
    }
}
