#![allow(clippy::unwrap_used)]

use sigil_reactive::Runtime;

use super::Session;

#[test]
fn write_without_spawn_errors() {
    Runtime::scope("pty-write", || {
        let session = Session::new();
        let err = session.write(b"x").unwrap_err();
        assert!(
            err.to_string().contains("spawn"),
            "expected spawn error, got {err}"
        );
    });
}

#[cfg(feature = "pty")]
use super::rows_of;

#[cfg(feature = "pty")]
#[test]
fn vt100_clear_and_print_lands_on_first_row() {
    let mut parser = vt100::Parser::new(24, 80, 0);
    parser.process(b"\x1b[2J\x1b[Hhello");
    let rows = rows_of(&parser);
    let first = rows.first().map(String::as_str).unwrap_or("");
    assert!(
        first.contains("hello"),
        "expected hello after CUP; got {rows:?}"
    );
}
