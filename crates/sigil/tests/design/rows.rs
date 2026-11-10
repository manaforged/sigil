use super::*;

const LONG: &str = "A sentence long enough to need more than one line in a narrow window";

#[test]
fn long_text_in_a_row_wraps_inside_the_window_and_pushes_later_widgets_down() {
    let mut ui = Harness::new(|| {
        column()
            .child(
                row()
                    .child(button("A"))
                    .child(text(LONG))
                    .child(button("B")),
            )
            .child(button("Below"))
    })
    .pixels(200, 400);
    let last = ui.get_by_role(Role::Button, "B").bounds().unwrap();
    let below = ui.get_by_role(Role::Button, "Below").bounds().unwrap();
    assert!(last.origin.x + last.width <= 200.5, "B ends at {last:?}");
    assert!(below.width <= 200.5, "the column is {} wide", below.width);
    assert!(
        below.origin.y >= last.origin.y + last.height,
        "{below:?} under {last:?}"
    );
    assert!(
        below.origin.y >= 3.0 * 16.0,
        "three lines of text end above {below:?}"
    );
}

#[test]
fn a_text_field_keeps_a_usable_width_inside_a_nested_row() {
    let mut ui = Harness::new(|| {
        row()
            .child(
                row()
                    .child(text_field(signal(String::new())))
                    .child(button("Go")),
            )
            .child(text("Words"))
    })
    .pixels(1280, 200);
    let field = ui
        .get_by_role(Role::TextInput, "Text field")
        .bounds()
        .unwrap();
    assert!(field.width >= 96.0, "the field is {} wide", field.width);
}

#[test]
fn a_button_keeps_its_label_when_a_field_fills_its_row() {
    let mut ui = Harness::new(|| {
        row()
            .child(text_field(signal(String::new())))
            .child(button("Save changes"))
    })
    .pixels(360, 200);
    let alone = {
        let mut lone = Harness::new(|| row().child(button("Save changes"))).pixels(360, 200);
        lone.get_by_role(Role::Button, "Save changes")
            .bounds()
            .unwrap()
            .width
    };
    let saved = ui
        .get_by_role(Role::Button, "Save changes")
        .bounds()
        .unwrap();
    assert!(
        (saved.width - alone).abs() < 0.5,
        "{} and {alone}",
        saved.width
    );
}

#[test]
fn every_button_fill_has_the_same_width() {
    let width = |fill: fn(ButtonBuilder) -> ButtonBuilder| {
        let mut ui = Harness::new(move || fill(button("Go"))).pixels(200, 80);
        ui.get_by_role(Role::Button, "Go").bounds().unwrap().width
    };
    let plain = width(|b| b);
    for (name, fill) in [
        (
            "primary",
            ButtonBuilder::primary as fn(ButtonBuilder) -> ButtonBuilder,
        ),
        ("outline", ButtonBuilder::outline),
        ("ghost", ButtonBuilder::ghost),
    ] {
        assert!((width(fill) - plain).abs() < 0.5, "{name}");
    }
}
