#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::hal::{Dim, LogicalSize};
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::reactive::Runtime;

struct Wrapping;

impl TextMeasure for Wrapping {
    fn measure(&self, _: &TextContent, space: sigil::hal::Size) -> LogicalSize {
        let w = match space.width {
            Dim::Fixed(w) => w.min(300.0),
            Dim::MinContent | Dim::MaxContent => 300.0,
        };
        LogicalSize::new(w, (300.0 / w).ceil() * 10.0)
    }
}

struct Swappable {
    width: Rc<Cell<f32>>,
    fonts: Rc<Cell<u64>>,
}

impl TextMeasure for Swappable {
    fn measure(&self, _: &TextContent, _: sigil::hal::Size) -> LogicalSize {
        LogicalSize::new(self.width.get(), 16.0)
    }

    fn font_generation(&self) -> u64 {
        self.fonts.get()
    }
}

fn leaf_rect(
    parent: LayoutStyle,
    leaf: LayoutStyle,
    measure: Box<dyn TextMeasure>,
) -> sigil::hal::Rect {
    Runtime::scope("layout-hardening", || {
        let mut cx = Cx::new_root();
        let root = cx.add(container().layout(parent));
        let label = cx.add_child(root, text("some words to wrap").layout(leaf));
        let mut engine = LayoutEngine::with_text_measure(measure);
        engine
            .compute(&mut cx, root, Size::fixed(400.0, 400.0))
            .unwrap();
        cx.get(label)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .expect("layout result")
            .rect()
    })
}

#[test]
fn non_finite_style_values_never_reach_a_rect() {
    for leaf in [
        LayoutStyle::block().width(f32::NAN),
        LayoutStyle::block().height(f32::INFINITY),
        LayoutStyle::block().padding(f32::NAN),
    ] {
        let rect = leaf_rect(
            LayoutStyle::block().fixed_size(200.0, 400.0),
            leaf,
            Box::new(Wrapping),
        );
        assert!(
            rect.width.is_finite() && rect.height.is_finite(),
            "{rect:?}"
        );
    }
}

#[test]
fn padded_text_in_a_flex_row_wraps_inside_its_row() {
    let rect = leaf_rect(
        LayoutStyle::flex_row().fixed_size(200.0, 400.0),
        LayoutStyle::block().padding(20.0),
        Box::new(Wrapping),
    );
    assert!((rect.width - 200.0).abs() < 0.5, "{rect:?}");
}

#[test]
fn a_fixed_width_wraps_text_at_its_content_box() {
    let rect = leaf_rect(
        LayoutStyle::block().fixed_size(400.0, 400.0),
        LayoutStyle::block().padding(20.0).width(100.0),
        Box::new(Wrapping),
    );
    assert_eq!((rect.width, rect.height), (100.0, 90.0));
}

#[test]
fn a_font_change_remeasures_text() {
    Runtime::scope("layout-fonts", || {
        let width = Rc::new(Cell::new(40.0));
        let fonts = Rc::new(Cell::new(0));
        let mut cx = Cx::new_root();
        let root = cx.add(container().layout(LayoutStyle::block().fixed_size(400.0, 40.0)));
        let label = cx.add_child(root, text("hello"));
        let mut engine = LayoutEngine::with_text_measure(Box::new(Swappable {
            width: Rc::clone(&width),
            fonts: Rc::clone(&fonts),
        }));
        let avail = Size::fixed(400.0, 40.0);
        engine.compute(&mut cx, root, avail).unwrap();
        width.set(90.0);
        fonts.set(1);
        cx.mark_mutated();
        engine.compute(&mut cx, root, avail).unwrap();
        let rect = cx
            .get(label)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .expect("layout result")
            .rect();
        assert!((rect.width - 90.0).abs() < 0.5, "{rect:?}");
    });
}
