#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::prelude::*;

struct ProportionalText;

impl TextMeasure for ProportionalText {
    fn measure(&self, content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        let len = match content {
            TextContent::Static(s) => s.len(),
            TextContent::Reactive(sig) => sig.peek().len(),
            TextContent::Compute(_) => content.resolve().len(),
        };
        #[allow(clippy::cast_precision_loss)]
        sigil::hal::LogicalSize::new(len as f32 * 7.0, 16.0)
    }
}

struct NativeLineGeometry;

impl sigil::layout::TextGeometry for NativeLineGeometry {
    fn hit(&self, _text: &str, _style: &sigil::style::TextStyle, _x: f32) -> usize {
        0
    }

    fn caret(
        &self,
        _text: &str,
        _style: &sigil::style::TextStyle,
        _byte: usize,
    ) -> sigil::layout::CaretGeometry {
        sigil::layout::CaretGeometry {
            x: 0.0,
            top: 0.0,
            height: 19.2,
        }
    }
}

#[derive(Debug)]
struct Violation {
    depth: usize,
    what: String,
}

const EPS: f32 = 0.5;

fn violations(cx: &Cx, layout: &LayoutEngine, root: NodeId) -> Vec<Violation> {
    let mut out = Vec::new();
    visit(cx, layout, root, (0, false), &mut out);
    out
}

type WalkState = (usize, bool);

fn visit(cx: &Cx, layout: &LayoutEngine, id: NodeId, state: WalkState, out: &mut Vec<Violation>) {
    let (depth, clipped) = state;
    let Some(node) = cx.get(id) else { return };
    let children: Vec<NodeId> = node.children().to_vec();
    let Some(result) = layout.result(id) else {
        return;
    };
    let rect = result.rect();
    let clips = result.clips();

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

const WIDTHS: [f32; 4] = [320.0, 640.0, 1100.0, 1600.0];

fn labeled_field(label: &'static str, control: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(4.0))
        .child(text(label))
        .child(control)
}

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

#[test]
fn compact_text_field_contains_native_line_height() {
    sigil::reactive::Runtime::scope("compact-text-field-layout", || {
        sigil::layout::set_text_geometry(std::rc::Rc::new(NativeLineGeometry));
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(signal(String::new())).width(560.0).height(34.0));
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalText));
        layout
            .compute(&mut cx, root, Size::fixed(560.0, 34.0))
            .expect("layout computes");

        let found = violations(&cx, &layout, root);
        assert!(
            found.is_empty(),
            "34px field must contain a 19.2px native line: {found:#?}"
        );
    });
}

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

#[test]
fn layout_re_resolves_when_the_container_resizes() {
    sigil::reactive::Runtime::scope("layout-invariants-resize", || {
        let mut cx = Cx::new_root();
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
