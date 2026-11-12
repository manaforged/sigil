#![allow(clippy::unwrap_used)]

use std::error::Error as StdError;
use std::fmt;

use sigil::{Error, ErrorKind};

#[derive(Debug)]
struct DummyErr(&'static str);

impl fmt::Display for DummyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl StdError for DummyErr {}

#[test]
fn error_new_wraps_arbitrary_std_error() {
    let e = Error::new(ErrorKind::Other, DummyErr("inner failure"));
    assert!(format!("{e}").contains("inner failure"));
}

#[test]
fn error_implements_std_error() {
    fn assert_std<E: StdError>(_: &E) {}
    let e = Error::new(ErrorKind::Other, DummyErr("x"));
    assert_std(&e);
}
