use std::collections::HashMap;

use crate::ThemeValue;

thread_local! {
    static CELL: core::cell::Cell<f32> = const { core::cell::Cell::new(0.0) };
}

pub fn grid(cell: f32) {
    CELL.with(|active| active.set(if cell > 0.0 { cell } else { 0.0 }));
}

pub(crate) fn tokens() -> HashMap<String, ThemeValue> {
    let cell = CELL.with(core::cell::Cell::get);
    if cell <= 0.0 {
        return HashMap::new();
    }
    let mut tokens = HashMap::new();
    for token in [
        "control-height-compact",
        "control-height-default",
        "control-height-wide",
        "padding-compact",
        "padding-default",
        "padding-wide",
        "gap-compact",
        "gap-default",
        "gap-wide",
    ] {
        tokens.insert(token.to_owned(), ThemeValue::Px(cell));
    }
    for token in ["radius-compact", "radius-default", "radius-wide"] {
        tokens.insert(token.to_owned(), ThemeValue::Px(0.0));
    }
    tokens
}
