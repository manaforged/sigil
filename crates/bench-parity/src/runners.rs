//! Per-framework runner implementations.
//!
//! Each framework lives in its own submodule. Adding a new framework
//! means: implement [`crate::FrameworkRunner`] in a fresh file under
//! this directory, re-export the type here, and register it in the
//! `bench-parity-runner` CLI dispatch.

mod construct;
mod quote_table;
mod sigil_runner;

pub use sigil_runner::SigilRunner;

/// Elapsed nanos since `start`, saturating at `u64::MAX`.
fn nanos(start: std::time::Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
