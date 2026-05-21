//! Per-framework runner implementations.
//!
//! Each framework lives in its own submodule. Adding a new framework
//! means: implement [`crate::FrameworkRunner`] in a fresh file under
//! this directory, re-export the type here, and register it in the
//! `bench-parity-runner` CLI dispatch.

mod sigil_runner;

pub use sigil_runner::SigilRunner;
