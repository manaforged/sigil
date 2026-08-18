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
//! SIGIL_PROFILE=1 cargo run --profile profiling -p shell-demo
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
mod tests {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use super::*;

    fn lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn disabled_span_is_silent() {
        let _gate = lock();
        disable();
        assert!(!enabled());
        {
            let _s = span("layout");
        }
        assert!(frames().is_empty());
    }

    #[test]
    fn span_records_when_enabled() {
        let _gate = lock();
        enable();
        reset();
        begin_frame();
        {
            let _s = span("layout");
            count("text.measure");
        }
        tick_frame();
        let got = frames();
        let last = got.last().expect("frame");
        assert!(
            last.phases.iter().any(|(n, _)| *n == "layout"),
            "layout phase present"
        );
        assert_eq!(last.counter("text.measure"), 1);
        disable();
    }

    #[test]
    fn pointer_beats_hover_on_the_same_frame() {
        let _gate = lock();
        enable();
        reset();
        begin_frame();
        interact("hover");
        interact("pointer");
        tick_frame();
        let got = frames();
        let last = got.last().expect("frame");
        assert_eq!(last.cause, "pointer");
        disable();
    }

    #[test]
    fn count_add_sums() {
        let _gate = lock();
        enable();
        reset();
        begin_frame();
        count_add("scene.cmds", 12);
        count_add("scene.cmds", 3);
        tick_frame();
        assert_eq!(frames().last().expect("frame").counter("scene.cmds"), 15);
        disable();
    }

    #[test]
    fn dump_writes_chrome_json() {
        let _gate = lock();
        enable();
        reset();
        begin_frame();
        interact("pointer");
        {
            let _s = span("paint");
            count("scene.emit");
        }
        let path = std::env::temp_dir().join("sigil-profile-dump-test.json");
        dump(&path).expect("dump");
        let json = std::fs::read_to_string(&path).expect("read dump");
        assert!(json.contains("\"traceEvents\""), "chrome envelope");
        assert!(json.contains("process_name"), "metadata");
        assert!(json.contains("\"name\":\"paint\""), "span name");
        assert!(json.contains("\"ph\":\"i\""), "input instant");
        assert!(json.contains("\"ph\":\"s\""), "flow start");
        assert!(json.contains("\"cat\":\"input\""), "input category");
        let text = summary();
        assert!(text.contains("paint"), "summary names paint");
        assert!(text.contains("pointer"), "summary names cause");
        assert!(
            text.lines()
                .any(|line| line.contains("scene.emit") && line.contains('1')),
            "counts print as integers: {text}"
        );
        disable();
        drop(std::fs::remove_file(path));
    }

    #[test]
    fn dump_closes_open_frame() {
        let _gate = lock();
        enable();
        reset();
        {
            let _s = span("effect");
        }
        assert!(frames().is_empty(), "not ticked yet");
        let path = std::env::temp_dir().join("sigil-profile-close-test.json");
        dump(&path).expect("dump");
        assert_eq!(frames().len(), 1, "open frame flushed");
        disable();
        drop(std::fs::remove_file(path));
    }
}
