//! Control chrome measured in terminal cells.
//!
//! A cell host paints on a character grid, where the row is the unit of
//! height. Desktop control metrics do not divide into it: a 28 px button on
//! an 8 px grid reserves four lines and paints its label somewhere in the
//! middle of them.
//!
//! Every widget builds its own [`Theme`](crate::Theme), so the host sets the
//! grid once here and the theme constructors seed their runtime tokens from
//! it, rather than the whole catalogue learning about terminals.

use std::collections::HashMap;

use crate::ThemeValue;

thread_local! {
    /// Cell size in logical pixels, or zero for desktop metrics.
    static CELL: core::cell::Cell<f32> = const { core::cell::Cell::new(0.0) };
}

/// Size control chrome to a `cell`-sized grid on this thread.
///
/// Controls then stand one cell tall, pad by one cell, and drop their corner
/// radius. Zero restores desktop metrics. Themes built after this call carry
/// the change; a cell host sets it before mounting a tree.
pub fn grid(cell: f32) {
    CELL.with(|active| active.set(if cell > 0.0 { cell } else { 0.0 }));
}

/// Runtime theme tokens for the active grid. Empty when none is set.
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
