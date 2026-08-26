//! Frame and interaction profiler for Sigil.
//!
//! Off until `SIGIL_PROFILE` is set (or [`enable`] is called). Then
//! every [`span`] / [`interact`] is recorded. On dump:
//!
//! * a Chrome / Perfetto JSON trace (nested spans, input instants,
//!   input→frame flows, counter tracks, ui + gpu threads)
//! * a stderr table: phases, per-cause interaction times, counters,
//!   and the frames that blew the 16.67 ms budget
//!
//! ```text
//! SIGIL_PROFILE=1 cargo run --profile profiling -p workbench
//! # writes ./sigil-profile.json — open in https://ui.perfetto.dev
//! ```
//!
//! When the env is unset, every call is one atomic load.

#![deny(missing_docs)]

mod chrome;
mod event;
mod report;
mod span;
mod store;

pub use span::Span;
pub use store::{FRAME_BUDGET_NS, Frame, INTERACT_BUDGET_NS, Phase};

/// Whether recording is on (`SIGIL_PROFILE` set and not `0`, or [`enable`]).
#[must_use]
pub fn enabled() -> bool {
    store::enabled()
}

/// Force recording on. Tests and benches use this.
pub fn enable() {
    store::enable();
}

/// Turn recording off and clear the log. Tests and benches use this.
pub fn disable() {
    store::disable();
}

/// Drop recorded frames and events. Does not change enabled.
pub fn reset() {
    store::reset();
}

/// Mark the start of a frame so wall-clock time is measured.
pub fn begin_frame() {
    store::begin_frame();
}

/// Open a named span. Drop ends it. No-op when disabled.
#[must_use]
pub fn span(name: &'static str) -> Span {
    Span::new(name)
}

/// Increment a counter on the current frame (`text.measure`, `path.raster`).
pub fn count(name: &'static str) {
    store::count_add(name, 1);
}

/// Add `n` to a counter on the current frame.
pub fn count_add(name: &'static str, n: u64) {
    store::count_add(name, n);
}

/// Record an interaction that caused this frame (`pointer`, `key`, `hover`).
///
/// Emits a Perfetto instant and starts an input→frame flow. A higher-rank
/// cause (click vs hover) replaces a weaker one on the same frame.
pub fn interact(name: &'static str) {
    store::interact(name);
}

/// Close the current frame and start the next.
pub fn tick_frame() {
    store::tick_frame();
}

/// Write the Chrome trace to `path` and print the summary.
///
/// Closes the open frame if it has events.
///
/// # Errors
///
/// Returns I/O errors from creating or writing `path`.
pub fn dump(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    store::close_open();
    let path = path.as_ref();
    chrome::write(path)?;
    eprintln!("{}", report::summary());
    eprintln!("sigil-profile: wrote {}", path.display());
    Ok(())
}

/// Dump if `SIGIL_PROFILE` is set. Path is the env value, or
/// `sigil-profile.json` when the value is `1`.
pub fn dump_if_enabled() {
    if let Some(path) = store::dump_path()
        && let Err(err) = dump(&path)
    {
        eprintln!("sigil-profile: dump failed: {err}");
    }
}

/// Closed frames (oldest first). Empty when disabled.
#[must_use]
pub fn frames() -> Vec<Frame> {
    store::frames()
}

/// Text summary of recorded frames.
#[must_use]
pub fn summary() -> String {
    report::summary()
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
