#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _c: &TextContent, _s: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(0.0, 0.0)
    }
}

struct ProportionalMeasure;

impl TextMeasure for ProportionalMeasure {
    fn measure(&self, content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        let len = match content {
            TextContent::Static(value) => value.len(),
            TextContent::Reactive(value) => value.peek().len(),
            TextContent::Compute(_) => content.resolve().len(),
        };
        let bounded = u16::try_from(len).unwrap_or(u16::MAX);
        sigil::hal::LogicalSize::new(f32::from(bounded) * 7.0, 16.0)
    }
}

fn widget_width<N: IntoNode + 'static>(parent_w: f32, make: impl FnOnce() -> N) -> f32 {
    let mut width = -1.0;
    sigil::reactive::Runtime::scope("width-override", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(parent_w, 200.0))
                .child(make()),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(parent_w, 200.0))
            .expect("layout");
        let child = *cx.get(root).unwrap().children().first().unwrap();
        width = layout.result(child).unwrap().rect().width;
    });
    width
}

#[test]
fn progress_fills_its_parent_by_default() {
    for parent in [100.0_f32, 244.0, 600.0] {
        let got = widget_width(parent, || progress(signal(0.5_f32), 1.0));
        assert!(
            (got - parent).abs() < 0.5,
            "progress should fill a {parent}px parent, got {got}",
        );
    }
}

#[test]
fn progress_width_pins_an_explicit_extent() {
    let sized = widget_width(600.0, || progress(signal(0.5_f32), 1.0).width(100.0));
    assert!(
        (sized - 100.0).abs() < 0.5,
        "progress.width(100) should measure 100 inside a 600px parent, got {sized}",
    );
}

#[test]
fn dropdown_fills_its_parent_by_default() {
    for parent in [120.0_f32, 212.0, 500.0] {
        let got = widget_width(parent, || {
            dropdown(signal(0_usize), vec!["alpha".to_owned()])
        });
        assert!(
            (got - parent).abs() < 0.5,
            "dropdown should fill a {parent}px parent, got {got}",
        );
    }
}

#[test]
fn dropdown_fits_a_narrow_parent_when_told_its_width() {
    let sized = widget_width(120.0, || {
        let opts = vec!["alpha".to_owned(), "beta".to_owned()];
        dropdown(signal(0_usize), opts).width(120.0)
    });
    assert!(
        (sized - 120.0).abs() < 0.5,
        "dropdown.width(120) should measure 120, got {sized}",
    );
}

#[test]
fn button_honours_its_width() {
    let sized = widget_width(300.0, || button("Save").width(140.0));
    assert!(
        (sized - 140.0).abs() < 0.5,
        "button.width(140) should measure 140, got {sized}",
    );
}

#[test]
fn hug_button_keeps_width_next_to_a_grow_spacer() {
    sigil::reactive::Runtime::scope("button-no-shrink", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(160.0, 40.0))
                .child(button("Create Task"))
                .child(container().layout(LayoutStyle::flex_row().grow(1.0)))
                .child(button("Start All")),
        );
        let children: Vec<_> = cx.get(root).expect("row").children().to_vec();
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(160.0, 40.0))
            .expect("layout");
        let create = layout
            .result(*children.first().expect("create"))
            .expect("create")
            .rect();
        let start = layout
            .result(*children.get(2).expect("start"))
            .expect("start")
            .rect();
        assert!(
            create.width >= 98.0,
            "Create Task must not shrink, got {}",
            create.width
        );
        assert!(
            start.width >= 84.0,
            "Start All must not shrink, got {}",
            start.width
        );
    });
}

#[test]
fn button_hugs_its_label_at_desktop_density() {
    sigil::reactive::Runtime::scope("button-hug", || {
        let mut cx = Cx::new_root();
        let root = cx.add(button("Save"));
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(300.0, 100.0))
            .expect("layout");
        let rect = layout.result(root).expect("button rect").rect();

        assert!(
            (rect.width - 50.0).abs() < 0.5,
            "expected 28 + 2×10 padding + 2×1 edge, got {rect:?}"
        );
        assert!(
            (rect.height - 28.0).abs() < 0.5,
            "expected 28px control, got {rect:?}"
        );
    });
}

#[test]
fn grow_size_fills_button_without_collapsing_its_height() {
    sigil::reactive::Runtime::scope("button-grow-size", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(300.0, 100.0))
                .child(button("Save").size(sigil::style::Size::Grow)),
        );
        let button = cx
            .get(root)
            .expect("root")
            .children()
            .first()
            .copied()
            .expect("button");
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(300.0, 100.0))
            .expect("layout");
        let rect = layout.result(button).expect("button rect").rect();

        assert!(
            (rect.width - 300.0).abs() < 0.5,
            "expected fill width, got {rect:?}"
        );
        assert!(
            (rect.height - 28.0).abs() < 0.5,
            "expected 28px control, got {rect:?}"
        );
    });
}

#[test]
fn text_field_inner_surface_leaves_one_pixel_frame() {
    sigil::reactive::Runtime::scope("text-field-frame", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            text_field(signal(String::from("Hello")))
                .width(600.0)
                .height(42.0),
        );
        let inner = cx
            .get(root)
            .expect("field")
            .children()
            .first()
            .copied()
            .expect("inner surface");
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(600.0, 42.0))
            .expect("layout");
        let outer = layout.result(root).expect("field rect").rect();
        let inner = layout.result(inner).expect("inner rect").rect();

        assert!(
            (inner.origin.y - outer.origin.y - 1.0).abs() < 0.5,
            "inner surface must start after the 1px frame: outer={outer:?} inner={inner:?}",
        );
        assert!(
            (inner.height - (outer.height - 2.0)).abs() < 0.5,
            "inner surface must fill the frame: outer={outer:?} inner={inner:?}",
        );
    });
}

#[test]
fn button_and_toggle_fill_via_grow() {
    let mut cx_widths = Vec::new();
    sigil::reactive::Runtime::scope("grow-fill", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(400.0, 60.0))
                .child(button("Save").grow(1.0))
                .child(toggle(signal(false)).grow(1.0)),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 60.0))
            .expect("layout");
        for child in cx.get(root).unwrap().children() {
            cx_widths.push(layout.result(*child).unwrap().rect().width);
        }
    });
    assert_eq!(
        cx_widths.len(),
        2,
        "expected button + toggle, got {cx_widths:?}"
    );
    let total: f32 = cx_widths.iter().sum();
    assert!(
        (total - 400.0).abs() < 0.5,
        "grow(1) controls should consume the full 400px row, got {cx_widths:?} \
         summing to {total}",
    );
}

#[test]
fn toggle_honours_its_width() {
    let sized = widget_width(300.0, || toggle(signal(false)).width(160.0));
    assert!(
        (sized - 160.0).abs() < 0.5,
        "toggle.width(160) should measure 160, got {sized}",
    );
}
