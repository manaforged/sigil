#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use std::error::Error as StdError;
use std::fmt;

use sigil_app::Error;

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
    let e = Error::new(DummyErr("inner failure"));
    assert!(format!("{e}").contains("inner failure"));
}

#[test]
fn error_renderer_error_into_works_via_question_mark() {
    fn try_renderer() -> Result<(), Error> {
        Err(sigil_renderer::RendererError::NotInitialized)?;
        Ok(())
    }
    let err = try_renderer().expect_err("must propagate");
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("init") || msg.to_lowercase().contains("surface"),
        "expected RendererError::NotInitialized message, got: {msg}"
    );
}

#[test]
fn error_implements_std_error() {
    fn assert_std<E: StdError>(_: &E) {}
    let e = Error::new(DummyErr("x"));
    assert_std(&e);
}
