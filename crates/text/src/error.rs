//! Text engine error types.

use core::fmt;

/// Errors returned by the Parley text engine.
///
/// Parley v0.8's layout path is infallible. This type exists as the
/// `TextEngine::Error` associated type and is currently uninhabited.
/// If a future Parley version introduces fallible layout, add a
/// variant here rather than changing the trait surface.
pub enum TextError {}

impl fmt::Debug for TextError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

impl std::error::Error for TextError {}
