#![allow(clippy::float_cmp, clippy::unwrap_used)]

use crate::layout_stub as stub;

use sigil::geometry::{Constraint, Size};
use sigil::kernel::{TextContent, *};
use sigil::layout::Space;
use sigil::layout::*;
use sigil::style::{Paint, TextLineHeight, TextStyle};
use sigil::widgets::Layout;

use stub::{first_child, layout_rect, stub_engine};

#[test]
fn p_4_applies_sixteen_px_padding() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::block()
                    .fixed_size(100.0, 50.0)
                    .padding(Space::Lg),
            )
            .child(
                container().layout(
                    LayoutStyle::default()
                        .width(Length::Percent(1.0))
                        .height(Length::Percent(1.0)),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Constraint::fixed(100.0, 50.0))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 16.0);
    assert_eq!(rect.origin.y, 16.0);
    assert_eq!(rect.width, 68.0);
    assert_eq!(rect.height, 18.0);
}

fn padded_child_origin(root: NodeBuilder) -> f32 {
    let mut cx = Cx::new_root();
    let root = cx.add(
        root.child(
            container().layout(
                LayoutStyle::default()
                    .width(Length::Percent(1.0))
                    .height(Length::Percent(1.0)),
            ),
        ),
    );
    let child = first_child(&cx, root);
    stub_engine()
        .compute(&mut cx, root, Constraint::fixed(100.0, 50.0))
        .expect("compute succeeds");
    layout_rect(&cx, child).origin.x
}

#[test]
fn a_class_written_after_layout_wins() {
    let origin = padded_child_origin(
        container()
            .layout(
                LayoutStyle::block()
                    .fixed_size(100.0, 50.0)
                    .padding(Space::Xl),
            )
            .layout(LayoutStyle::default().padding(Space::Lg)),
    );
    assert_eq!(origin, 16.0);
}

#[test]
fn a_layout_written_after_class_wins() {
    let origin = padded_child_origin(
        container()
            .layout(LayoutStyle::default().padding(Space::Lg))
            .layout(
                LayoutStyle::block()
                    .fixed_size(100.0, 50.0)
                    .padding(Space::Xl),
            ),
    );
    assert_eq!(origin, 24.0);
}

#[test]
fn w_full_is_percent_100() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 40.0))
            .child(
                container().layout(
                    LayoutStyle::default()
                        .height(10.0)
                        .width(Length::Percent(1.0)),
                ),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Constraint::fixed(200.0, 40.0))
        .expect("compute succeeds");

    assert_eq!(layout_rect(&cx, child).width, 200.0);
}

#[test]
fn flex_row_items_center() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::default()
                    .fixed_size(100.0, 40.0)
                    .display(Display::Flex)
                    .flex_direction(FlexDirection::Row)
                    .align_items(AlignItems::Center),
            )
            .child(container().layout(LayoutStyle::default().fixed_size(10.0, 10.0)))
            .child(container().layout(LayoutStyle::default().fixed_size(20.0, 10.0))),
    );
    let children = cx.get(root).unwrap().children().to_vec();

    stub_engine()
        .compute(&mut cx, root, Constraint::fixed(100.0, 40.0))
        .expect("compute succeeds");

    let first = layout_rect(&cx, *children.first().unwrap());
    let second = layout_rect(&cx, *children.get(1).unwrap());

    assert_eq!(first.origin.x, 0.0);
    assert_eq!(first.origin.y, 15.0);
    assert_eq!(second.origin.x, 10.0);
    assert_eq!(second.origin.y, 15.0);
}

#[test]
fn typography_atoms_drive_text_measurement() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        text("Type")
            .font_size(20.0, "test fixture")
            .font_weight(700)
            .line_height(32.0),
    );

    LayoutEngine::new(Box::new(StyleAwareMeasure))
        .compute(&mut cx, root, Constraint::max_content())
        .expect("compute succeeds");

    let rect = layout_rect(&cx, root);
    assert_eq!(rect.width, 27.0);
    assert_eq!(rect.height, 32.0);
}

struct StyleAwareMeasure;

impl TextMeasure for StyleAwareMeasure {
    fn measure(&self, _content: &TextContent, _space: Constraint) -> Size {
        Size::default()
    }

    fn measure_styled(
        &self,
        _content: &TextContent,
        style: &TextStyle,
        _space: Constraint,
    ) -> Size {
        Size::new(
            style.font_size + f32::from(style.font_weight) / 100.0,
            text_line_height(style),
        )
    }
}

fn text_line_height(style: &TextStyle) -> f32 {
    match style.line_height {
        TextLineHeight::FontSizeRelative(value) => style.font_size * value,
        TextLineHeight::Absolute(value) => value,
    }
}
