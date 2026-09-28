#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::prelude::*;
use sigil::scene::{SceneCommand, emit_scene};
use sigil_kittest::CharMeasure;

const TEXT_MEASURE: CharMeasure = CharMeasure {
    char_width: 7.0,
    line_height: 16.0,
};

fn painted_text(width: f32, build: impl FnOnce() -> NodeBuilder) -> Vec<String> {
    let mut found = Vec::new();
    sigil::reactive::Runtime::scope("text-presence", || {
        let mut cx = Cx::new_root();
        let root = cx.add(build());
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(width, 400.0))
            .expect("layout computes");
        found = emit_scene(&cx, root)
            .commands
            .iter()
            .filter_map(|c| match c {
                SceneCommand::GlyphRun(g) if g.run.rect.width > 0.5 && g.run.rect.height > 0.5 => {
                    Some(g.run.text.clone())
                }
                _ => None,
            })
            .collect();
    });
    found
}

fn assert_paints(width: f32, needle: &str, build: impl FnOnce() -> NodeBuilder) {
    let painted = painted_text(width, build);
    assert!(
        painted.iter().any(|t| t.contains(needle)),
        "at width {width}: expected some glyph run containing {needle:?}, \
         but the scene painted {painted:?}",
    );
}

const WIDTHS: [f32; 3] = [140.0, 320.0, 900.0];

#[test]
fn dropdown_paints_its_selected_option() {
    for width in WIDTHS {
        assert_paints(width, "Banana", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(dropdown(
                    signal(0_usize),
                    vec!["Banana".into(), "Cherry".into()],
                ))
        });
    }
}

#[test]
fn button_paints_its_label() {
    for width in WIDTHS {
        assert_paints(width, "Save", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(button("Save"))
        });
    }
}

#[test]
fn text_field_paints_its_value() {
    for width in WIDTHS {
        assert_paints(width, "hello", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(text_field(signal(String::from("hello"))))
        });
    }
}

#[test]
fn checkbox_paints_its_label() {
    for width in WIDTHS {
        assert_paints(width, "enabled", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(checkbox(signal(true)).label("enabled"))
        });
    }
}

#[test]
fn text_survives_the_equal_columns_idiom() {
    for width in WIDTHS {
        let painted = painted_text(width, || {
            fn cell(child: impl IntoNode + 'static) -> NodeBuilder {
                container()
                    .layout(LayoutStyle::flex_column().grow(1.0).width(0.0))
                    .child(child)
            }
            container()
                .layout(LayoutStyle::flex_row().fill_width().gap(8.0))
                .child(cell(dropdown(signal(0_usize), vec!["Banana".into()])))
                .child(cell(button("Save")))
        });
        for needle in ["Banana", "Save"] {
            assert!(
                painted.iter().any(|t| t.contains(needle)),
                "at width {width}: {needle:?} vanished inside a flex:1 1 0 cell; \
                 scene painted {painted:?}",
            );
        }
    }
}
