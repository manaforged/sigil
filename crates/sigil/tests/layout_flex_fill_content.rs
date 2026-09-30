#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil_kittest::Harness;

fn in_block(content: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(400.0))
        .child(content)
}

fn button_span(ui: &mut Harness, name: &str) -> (f32, f32) {
    ui.settle();
    let bounds = ui
        .get_by_role(Role::Button, name)
        .bounds()
        .expect("the button is laid out");
    (bounds.origin.x, bounds.origin.x + bounds.width)
}

#[test]
fn a_spacer_pushes_the_last_child_to_the_end_of_the_row() {
    let mut ui = Harness::new(|| {
        in_block(
            row()
                .child(text("Title"))
                .child(spacer())
                .child(button("Close")),
        )
    })
    .pixels(800, 200);
    let (_, end) = button_span(&mut ui, "Close");
    assert!((end - 400.0).abs() < 0.5, "Close ends at {end}");
}

#[test]
fn a_row_that_fills_shares_its_parent_row_without_overlap() {
    let mut ui = Harness::new(|| {
        let name = signal(String::new());
        in_block(
            row()
                .child(row().child(text_field(name)).child(button("Add")))
                .child(button("Settings")),
        )
    })
    .pixels(800, 200);
    let (_, add_end) = button_span(&mut ui, "Add");
    let (settings_start, settings_end) = button_span(&mut ui, "Settings");
    assert!(
        add_end <= settings_start,
        "Add ends at {add_end}, Settings starts at {settings_start}"
    );
    assert!(settings_end <= 400.5, "Settings ends at {settings_end}");
}

#[test]
fn a_column_with_a_filling_row_stays_inside_its_parent() {
    let mut ui = Harness::new(|| {
        let name = signal(String::new());
        in_block(column().child(row().child(text_field(name)).child(button("Add"))))
    })
    .pixels(800, 200);
    let (_, end) = button_span(&mut ui, "Add");
    assert!(end <= 400.5, "Add ends at {end}");
}

#[test]
fn a_button_in_a_column_keeps_its_content_width() {
    let mut ui = Harness::new(|| in_block(column().child(button("Save")))).pixels(800, 200);
    let (start, end) = button_span(&mut ui, "Save");
    assert!(end - start < 100.0, "Save spans {start}..{end}");
}
