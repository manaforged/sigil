//! Benchmark spec shared by every framework runner.
//!
//! A runner drives a real window through [`ledger::script`], stamps
//! `Instant::now()` on every 16 ms pulse, and writes a [`Record`]. The
//! report tool reads records only; it never measures.

#![allow(missing_docs)]

pub mod ledger;
mod record;

pub use record::{BUDGET_60_NS, BUDGET_120_NS, Check, Host, Record, Summary, summarize};

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
