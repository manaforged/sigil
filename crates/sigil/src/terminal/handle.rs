use std::io::Write;

use super::{Activity, Backdrop, Capabilities, MouseTracking};
use crate::reactive::Signal;

/// The terminal that the current thread drives.
#[derive(Clone, Copy, Debug)]
pub struct Terminal(());

impl Terminal {
    /// Returns the handle of the terminal that the current thread drives.
    #[must_use]
    pub const fn current() -> Self {
        Self(())
    }

    /// Sets the terminal title; it is written at once when the terminal host is live.
    pub fn title(self, title: &str) -> Result<(), crate::Error> {
        super::term::title(title)
    }

    /// Sets how much mouse movement the terminal reports.
    pub fn mouse_tracking(self, tracking: MouseTracking) -> Result<(), crate::Error> {
        super::term::mouse_tracking(tracking)
    }

    /// Whether the terminal host draws the window layout as images.
    #[must_use]
    pub fn pixels(self) -> bool {
        super::pixels::pixels()
    }

    /// Asks the terminal host to draw the window layout as images, and returns whether it will.
    #[expect(
        clippy::must_use_candidate,
        reason = "callers switch the mode for its effect"
    )]
    pub fn draw_pixels(self, on: bool) -> bool {
        super::pixels::set_pixels(on)
    }

    /// Writes lines above the inline live region, into scrollback.
    pub fn print_above(self, text: &str) -> Result<(), crate::Error> {
        super::term::print_above(text)
    }

    /// Sends a desktop notification through the terminal.
    pub fn notify(self, title: &str, body: &str) -> Result<(), crate::Error> {
        super::notify::notify(title, body)
    }

    /// Reports task progress to the terminal; `None` clears it.
    pub fn progress(self, value: Option<u8>) -> Result<(), crate::Error> {
        super::notify::progress(value)
    }

    /// Rings the terminal bell.
    pub fn bell(self, note: Option<&str>) -> Result<(), crate::Error> {
        super::notify::bell(note)
    }

    /// Reports the activity of `app` to the host; `None` clears it.
    pub fn activity(self, app: &str, value: Option<Activity>) -> Result<(), crate::Error> {
        super::notify::activity(app, value)
    }

    /// Measures the width of ambiguous characters in this terminal.
    pub fn calibrate(self, out: &mut impl Write) -> Result<(), crate::Error> {
        super::run::calibrate(out)
    }

    /// Leaves terminal mode, runs `run`, and returns to terminal mode.
    pub fn suspend<R>(self, run: impl FnOnce() -> R) -> Result<R, crate::Error> {
        super::term::suspend(run)
    }

    /// Returns whether the terminal window has input focus, as the terminal last reported it.
    #[must_use]
    pub fn is_focused(self) -> bool {
        super::term::focused()
    }

    /// The capabilities that the host probed when it entered raw mode.
    #[must_use]
    pub fn caps(self) -> Option<Capabilities> {
        super::term::caps()
    }

    /// The background brightness, updated when the system light or dark setting changes.
    #[must_use]
    pub fn backdrop(self) -> Signal<Option<Backdrop>> {
        super::backdrop::backdrop()
    }
}
