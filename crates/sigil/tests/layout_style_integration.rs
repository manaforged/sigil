#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;

use sigil::kernel::TextContent;
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::style::NodeStyleExt;
use sigil::style::{TextLineHeight, TextStyle};

use stub::stub_engine;

#[test]
fn p_4_applies_sixteen_px_padding() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 50.0))
            .class("p-4")
            .child(container().class("w-full h-full")),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 50.0))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 16.0);
    assert_eq!(rect.origin.y, 16.0);
    assert_eq!(rect.width, 68.0);
    assert_eq!(rect.height, 18.0);
}

#[test]
fn layout_style_padding_wins_over_class() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 50.0).padding(24.0))
            .class("p-4")
            .child(container().class("w-full h-full")),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 50.0))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 24.0);
    assert_eq!(rect.origin.y, 24.0);
    assert_eq!(rect.width, 52.0);
    assert_eq!(rect.height, 2.0);
}

#[test]
fn w_full_is_percent_100() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 40.0))
            .child(
                container()
                    .layout(LayoutStyle::default().height(10.0))
                    .class("w-full"),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 40.0))
        .expect("compute succeeds");

    assert_eq!(layout_rect(&cx, child).width, 200.0);
}

#[test]
fn flex_row_items_center() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::default().fixed_size(100.0, 40.0))
            .class("flex flex-row items-center")
            .child(container().layout(LayoutStyle::default().fixed_size(10.0, 10.0)))
            .child(container().layout(LayoutStyle::default().fixed_size(20.0, 10.0))),
    );
    let children = cx.get(root).unwrap().children().to_vec();

    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 40.0))
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
    let root = cx.add(text("Type").class("text-xl font-bold leading-8"));

    LayoutEngine::with_text_measure(Box::new(StyleAwareMeasure))
        .compute(&mut cx, root, Size::max_content())
        .expect("compute succeeds");

    let rect = layout_rect(&cx, root);
    assert_eq!(rect.width, 27.0);
    assert_eq!(rect.height, 32.0);
}

fn first_child(cx: &Cx, root: NodeId) -> NodeId {
    cx.get(root).unwrap().children().first().copied().unwrap()
}

fn layout_rect(cx: &Cx, id: NodeId) -> sigil::hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
}

struct StyleAwareMeasure;

impl TextMeasure for StyleAwareMeasure {
    fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::default()
    }

    fn measure_styled(
        &self,
        _content: &TextContent,
        style: &TextStyle,
        _space: Size,
    ) -> LogicalSize {
        LogicalSize::new(
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
