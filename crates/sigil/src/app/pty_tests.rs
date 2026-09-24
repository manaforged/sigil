#![allow(clippy::unwrap_used)]

use super::rows_of;

#[test]
fn vt100_clear_and_print_lands_on_first_row() {
    let mut parser = vt100::Parser::new(24, 80, 0);
    parser.process(b"\x1b[2J\x1b[Hhello");
    let rows = rows_of(&parser);
    let first = rows.first().map_or("", String::as_str);
    assert!(
        first.contains("hello"),
        "expected hello after CUP; got {rows:?}"
    );
}
