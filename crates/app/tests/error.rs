//! `sigil_app::Error` wrapping behaviour.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use crossbeam_channel as _;
use muda as _;
use notify as _;
use raw_window_handle as _;
use rfd as _;
use sigil_a11y as _;
use sigil_animation_runtime as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_tasks as _;
use sigil_text as _;
use toml as _;
use tracing as _;
use tray_icon as _;
#[cfg(windows)]
use windows_core as _;

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
    // The renderer error path is the main reason `?` should succeed
    // inside `launch::run` — make sure the From impl exists.
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
