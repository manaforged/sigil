use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::{Harness, Query};

#[test]
fn committing_text_that_does_not_parse_shows_the_stored_value() {
    Owner::scope("number-commit", || {
        let value = signal(0.0_f64);
        let mut ui = Harness::new(move || column().child(number_field(value, 0.0..=1_000_000.0)))
            .pixels(400, 120);
        ui.settle();
        let input = ui.get(&Query::role(Role::TextInput));
        ui.click(&input)
            .type_text("1.234,56")
            .press(Key::Enter)
            .settle();
        let shown = ui
            .get(&Query::role(Role::TextInput))
            .value()
            .map(str::to_owned);
        assert_eq!(shown, Some(value.get().to_string()));
    });
}

fn commit(value_field: impl IntoNode + 'static, text: &str) -> Harness {
    let mut ui = Harness::new(move || column().child(value_field)).pixels(400, 120);
    ui.settle();
    let input = ui.get(&Query::role(Role::TextInput));
    ui.click(&input).press(Key::End);
    for _ in 0..24 {
        ui.press(Key::Backspace);
    }
    ui.type_text(text).press(Key::Enter).settle();
    ui.focus_next().settle();
    ui
}

fn shown(ui: &mut Harness) -> Option<String> {
    ui.get(&Query::role(Role::TextInput))
        .value()
        .map(str::to_owned)
}

#[test]
fn committing_a_number_past_the_range_shows_the_clamped_value() {
    Owner::scope("number-clamp", || {
        let value = signal(50_i32);
        let mut ui = commit(number_field(value, 0..=100), "500");
        assert_eq!(
            (value.peek(), shown(&mut ui)),
            (100, Some("100".to_owned()))
        );
    });
}

#[test]
fn committing_a_number_shows_its_canonical_text() {
    Owner::scope("number-canonical", || {
        let value = signal(1.0_f64);
        let mut ui = commit(number_field(value, 0.0..=10.0), "1.50");
        assert_eq!(
            (value.peek(), shown(&mut ui)),
            (1.5, Some("1.5".to_owned()))
        );
    });
}
