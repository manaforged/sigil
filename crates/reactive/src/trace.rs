//! Effect trace log behind `SIGIL_TRACE_EFFECTS=1`.
//!
//! DR-005 addendum item 6 promises "every effect firing with its debug
//! name, the signals read, and the signals written". This module is
//! the hook: [`crate::effect::run_effect`] calls [`trace_effect_fire`]
//! at the end of every successful run and — when the env flag is set
//! — one line goes to stderr in the format documented below.
//!
//! # Output format (public, parseable)
//!
//! ```text
//! [sigil trace] effect fired: name=<effect_name> at=<file>:<line> reads=[<N>] writes_pending=[<N>]
//! ```
//!
//! - `name` is the effect's `&'static str` name (via
//!   [`crate::effect::effect_named`]; `effect()` registers the literal
//!   `"<unnamed>"`).
//! - `at` is the `file:line` captured at registration by
//!   `#[track_caller]`.
//! - `reads` is `deps.len()` after the run — the exact number of
//!   signals the closure subscribed to this run. The dep list is
//!   cleared and rebuilt on every run, so this is the fresh set, not
//!   a cumulative one.
//! - `writes_pending` is the number of effects currently queued in
//!   `rt.pending` after this run enqueued any dependents. Reports `0`
//!   when we are not inside a batch (effects ran synchronously rather
//!   than queueing).
//!
//! Downstream tooling (inspector, test helpers, third-party dev tools)
//! can parse this shape; treat it as semi-stable output.
//!
//! # Zero-cost opt-in
//!
//! The env var is read exactly once per process via a [`OnceLock<()>`]
//! that populates an [`AtomicBool`]. Every subsequent call to
//! [`trace_effect_fire`] is a single relaxed atomic load; when the
//! flag is unset, the function returns before any formatting runs.
//! Release builds that never set `SIGIL_TRACE_EFFECTS` or the legacy
//! `RUST_UI_TRACE_EFFECTS` alias pay one relaxed load per effect fire
//! and nothing else.
//!
//! # Test hook
//!
//! Setting `SIGIL_TRACE_EFFECTS` from inside a test is flaky —
//! `OnceLock` latches the env value at first check and other tests in
//! the same process may fire effects before/after the set. The
//! [`test_hook`] submodule therefore exposes a per-thread capture
//! buffer: tests call [`test_hook::enable_capture`], run their code,
//! and then [`test_hook::take_last_line`] to inspect the formatted
//! output without touching the env. The capture side runs
//! regardless of the env flag so the "flag off" path is observable
//! too.

use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Sticky cache of the trace env check. Initialized
/// lazily through [`ensure_env_checked`] so the first effect fire pays
/// the env read; every subsequent fire pays one relaxed load.
static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Marker that the env var has been checked. `OnceLock<()>` is the
/// simplest std primitive for a one-shot initializer with no payload;
/// the payload is the `TRACE_ENABLED` atomic itself.
static ENV_CHECKED: OnceLock<()> = OnceLock::new();

/// Read trace opt-in environment variables at most once per process and
/// cache the result in [`TRACE_ENABLED`].
///
/// The contract is: "`=1` enables the trace, anything else (including
/// unset) disables it". We intentionally do not accept `true` / `yes`
/// / other truthy spellings — a single documented on-switch is easier
/// to script against and easier to remember.
fn ensure_env_checked() {
    ENV_CHECKED.get_or_init(|| {
        let sigil_enabled =
            std::env::var_os("SIGIL_TRACE_EFFECTS").as_deref() == Some("1".as_ref());
        let legacy_enabled =
            std::env::var_os("RUST_UI_TRACE_EFFECTS").as_deref() == Some("1".as_ref());
        if sigil_enabled || legacy_enabled {
            TRACE_ENABLED.store(true, Ordering::Relaxed);
        }
    });
}

/// Fast pre-check that runs before the caller pays the cost of
/// snapshotting effect-slot fields for the trace line.
///
/// Returns `true` iff either the env flag is set *or* a test on this
/// thread has enabled the capture buffer — the only two reasons to
/// bother building the trace line. Keeps the hot path (flag off,
/// no tests) down to one atomic load + one thread-local read before
/// the caller can bail out.
#[inline]
pub(crate) fn trace_active() -> bool {
    ensure_env_checked();
    if TRACE_ENABLED.load(Ordering::Relaxed) {
        return true;
    }
    // The test-capture branch is always compiled but is a single
    // thread-local `Cell::get` — effectively free on the hot path.
    // Keeping it unconditional means integration tests (which live
    // in a separate crate and so do not inherit `#[cfg(test)]`) can
    // still drive the capture buffer.
    test_hook::capture_enabled()
}

/// Emit (or not) the one-line trace for an effect that just fired.
///
/// Called from [`crate::effect::run_effect`] after the closure has
/// returned and the runtime state has settled. Callers first check
/// [`trace_active`] to gate the runtime-touching snapshot; by the
/// time we get here at least one sink (stderr or the test buffer)
/// wants the line.
///
/// # Parameters
///
/// - `name`: the effect's debug name, `&'static str`.
/// - `location`: the `#[track_caller]` location captured at
///   `effect()` / `effect_named()` registration.
/// - `reads_count`: size of the effect's post-run dep set
///   (`effect.deps.len()`).
/// - `writes_pending_count`: size of the pending-effect queue right
///   after this run scheduled its subscribers. `0` outside a batch.
pub(crate) fn trace_effect_fire(
    name: &'static str,
    location: &'static core::panic::Location<'static>,
    reads_count: usize,
    writes_pending_count: usize,
) {
    // `trace_active` was called by the caller; the env has been
    // checked at least once. Re-check the atomic cheaply so we do
    // not emit to stderr from inside a test that only enabled the
    // capture buffer.
    if TRACE_ENABLED.load(Ordering::Relaxed) {
        // eprintln! so the trace works without any logging framework
        // attached, and so it stays on stderr where diagnostic output
        // belongs (never confused with application stdout).
        eprintln!(
            "[sigil trace] effect fired: name={name} at={file}:{line} reads=[{reads_count}] writes_pending=[{writes_pending_count}]",
            file = location.file(),
            line = location.line(),
        );
    }

    test_hook::maybe_record(name, location, reads_count, writes_pending_count);
}

/// Per-thread test capture hook.
///
/// Hidden from rustdoc and the public API surface — callers outside
/// the crate's own tests have no reason to use this. The hook lives
/// in its own submodule so the `pub` items are explicitly scoped and
/// obvious in a grep for "public API".
#[doc(hidden)]
pub mod test_hook {
    use std::cell::{Cell, RefCell};

    thread_local! {
        /// Last trace line formatted on this thread, or `None`.
        static LAST_TRACE: RefCell<Option<String>> = const { RefCell::new(None) };
        /// Opt-in switch for the capture buffer. Tests that want to
        /// observe trace lines call [`enable_capture`] first;
        /// otherwise the capture is a single `Cell::get` no-op.
        static CAPTURE_ENABLED: Cell<bool> = const { Cell::new(false) };
    }

    /// Enable the per-thread capture buffer for this thread.
    ///
    /// Idempotent. Call once at the top of a test that wants to
    /// observe trace lines.
    pub fn enable_capture() {
        CAPTURE_ENABLED.with(|cell| cell.set(true));
    }

    /// Fast-path accessor used by [`super::trace_active`] to decide
    /// whether to proceed with a trace snapshot when the env flag is
    /// off. One thread-local `Cell::get` — effectively free on the
    /// hot path.
    pub(crate) fn capture_enabled() -> bool {
        CAPTURE_ENABLED.with(Cell::get)
    }

    /// Disable the per-thread capture buffer and clear it.
    pub fn disable_capture() {
        CAPTURE_ENABLED.with(|cell| cell.set(false));
        reset_last_line();
    }

    /// Take and clear the most recent captured trace line on this
    /// thread.
    #[must_use]
    pub fn take_last_line() -> Option<String> {
        LAST_TRACE.with(|slot| slot.borrow_mut().take())
    }

    /// Clear the last-line buffer without returning its contents.
    pub fn reset_last_line() {
        LAST_TRACE.with(|slot| *slot.borrow_mut() = None);
    }

    /// Internal: record a line iff capture is enabled on this thread.
    pub(crate) fn maybe_record(
        name: &'static str,
        location: &'static core::panic::Location<'static>,
        reads_count: usize,
        writes_pending_count: usize,
    ) {
        if !CAPTURE_ENABLED.with(Cell::get) {
            return;
        }
        let line = format!(
            "[sigil trace] effect fired: name={name} at={file}:{line} reads=[{reads_count}] writes_pending=[{writes_pending_count}]",
            file = location.file(),
            line = location.line(),
        );
        LAST_TRACE.with(|slot| *slot.borrow_mut() = Some(line));
    }
}
