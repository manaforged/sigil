#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::{Harness, Query};

#[test]
fn committing_text_that_does_not_parse_shows_the_stored_value() {
    Runtime::scope("number-commit", || {
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
