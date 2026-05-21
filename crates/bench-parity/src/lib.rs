//! Cross-framework bench-parity harness.
//!
//! Locks a workload (row count, viewport, scroll pattern, equivalence
//! check) in JSON so multiple frameworks can be measured against the
//! same definition. The harness ONLY produces per-framework results;
//! comparative claims live in the aggregator and require that every
//! framework's equivalence checks pass before any "X vs Y" line is
//! emitted.
//!
//! This crate is intentionally NOT in `default-members` — the
//! eventual `GPUI` / `TanStack` runners drag in heavy dependency trees
//! we do not want polluting `cargo build` from the workspace root.

#![allow(clippy::module_name_repetitions)]

// Bins under `src/bin/` re-pull these via the lib. Silence the
// unused-extern-crate lint at the lib level so the bins' transitive
// use is the canonical use site.
use clap as _;
use serde_json as _;
use time as _;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub mod runners;

pub use runners::SigilRunner;

/// One bench workload — a frozen description of what is measured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    /// Stable identifier — file name without extension, also the
    /// grouping key in the aggregator report.
    pub workload_id: String,
    /// Human description, surfaces in the markdown report header.
    pub description: String,
    /// Workload category. The runner dispatches on this string.
    pub category: String,
    /// Typed parameter block for the named category.
    pub params: WorkloadParams,
    /// Required output-equivalence checks. Every framework runner
    /// must report Pass or Fail (with detail) for each entry.
    #[serde(default)]
    pub equivalence_checks: Vec<EquivalenceCheckSpec>,
}

/// Workload parameters, currently shaped for `category == "virtual-list"`.
///
/// Additional categories should add fields here behind serde defaults
/// rather than introducing a parallel struct — keeps the JSON schema
/// extensible without breaking older workload files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadParams {
    /// Total number of rows in the data source.
    #[serde(default)]
    pub row_count: usize,
    /// Uniform row height in logical pixels.
    #[serde(default)]
    pub row_height_px: f32,
    /// Viewport width in logical pixels.
    #[serde(default)]
    pub viewport_width_px: f32,
    /// Viewport height in logical pixels.
    #[serde(default)]
    pub viewport_height_px: f32,
    /// Row shape identifier (e.g. `krausest-4-cell`).
    #[serde(default)]
    pub row_shape: String,
    /// Scroll pattern identifier (e.g. `linear-one-row-per-frame`).
    #[serde(default)]
    pub scroll_pattern: String,
    /// Number of measured iterations.
    #[serde(default)]
    pub iterations: usize,
    /// Number of warmup iterations (not included in summary).
    #[serde(default)]
    pub warmup: usize,
}

/// Declared equivalence check the runner must answer Pass/Fail for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceCheckSpec {
    /// Stable check identifier — keys into [`RunResult::equivalence_check_results`].
    pub name: String,
    /// Human description, copied into the aggregator's report.
    pub description: String,
}

/// Runner-emitted verdict on a declared equivalence check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EquivalenceCheck {
    /// Check passed.
    Pass,
    /// Check failed; `detail` carries the divergence summary.
    Fail {
        /// Human-readable explanation of the divergence.
        detail: String,
    },
    /// Runner does not implement this check (e.g. layout-only runner
    /// against a paint-equivalence check).
    NotApplicable,
}

/// Per-runner result of one workload execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Framework identifier, matches `--framework` on the runner CLI.
    pub framework: String,
    /// Echoed [`Workload::workload_id`] for grouping in the aggregator.
    pub workload_id: String,
    /// Raw per-iteration durations in nanoseconds.
    pub samples_ns: Vec<u64>,
    /// Aggregate summary, computed from [`Self::samples_ns`].
    pub summary: RunSummary,
    /// Outcome of each declared equivalence check.
    pub equivalence_check_results: HashMap<String, EquivalenceCheck>,
    /// Host info captured at run time.
    pub host: HostInfo,
}

/// Aggregate statistics over the measured samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    /// Number of measured samples.
    pub iterations: usize,
    /// Median in microseconds.
    pub median_us: f64,
    /// p95 in microseconds.
    pub p95_us: f64,
    /// p99 in microseconds.
    pub p99_us: f64,
    /// Mean in microseconds.
    pub mean_us: f64,
}

impl RunSummary {
    /// Compute summary from raw nanosecond samples.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn from_samples(samples_ns: &[u64]) -> Self {
        if samples_ns.is_empty() {
            return Self {
                iterations: 0,
                median_us: 0.0,
                p95_us: 0.0,
                p99_us: 0.0,
                mean_us: 0.0,
            };
        }
        let mut sorted: Vec<u64> = samples_ns.to_vec();
        sorted.sort_unstable();
        let n = sorted.len();
        let last = n - 1;
        let pct = |p: f64| -> f64 {
            // Bench sample counts in this harness top out at the
            // thousands; the f64 mantissa precision warnings do not
            // apply at that scale and are explicitly allowed above.
            let rank = (p * (last as f64)).round() as usize;
            let idx = rank.min(last);
            sorted.get(idx).copied().unwrap_or(0) as f64 / 1_000.0
        };
        let sum: u128 = sorted.iter().map(|&v| u128::from(v)).sum();
        let mean_us = (sum as f64) / (n as f64) / 1_000.0;
        Self {
            iterations: n,
            median_us: pct(0.50),
            p95_us: pct(0.95),
            p99_us: pct(0.99),
            mean_us,
        }
    }
}

/// Host information captured per run for apples-to-apples joining.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    /// OS name + version (e.g. `Windows 11 Pro 26200`).
    pub os: String,
    /// CPU brand string.
    pub cpu: String,
    /// Logical CPU core count.
    pub cpu_cores: usize,
    /// Total system memory in bytes.
    pub total_memory_bytes: u64,
    /// rustc version (`rustc --version` minus newline), or `"unknown"`.
    pub rustc_version: String,
}

impl HostInfo {
    /// Probe the running host. Uses `sysinfo` for CPU+OS+memory.
    #[must_use]
    pub fn capture() -> Self {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let cpu = sys
            .cpus()
            .first()
            .map_or_else(|| "unknown".to_string(), |c| c.brand().to_string());
        let cpu_cores = sys.cpus().len();
        let total_memory_bytes = sys.total_memory();
        let os = format!(
            "{} {}",
            System::name().unwrap_or_else(|| "unknown".into()),
            System::os_version().unwrap_or_default(),
        )
        .trim()
        .to_string();
        let rustc_version = rustc_version_string();
        Self {
            os,
            cpu,
            cpu_cores,
            total_memory_bytes,
            rustc_version,
        }
    }
}

fn rustc_version_string() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

/// Trait implemented per framework. Runner is stateless; all
/// inputs flow through the workload.
pub trait FrameworkRunner {
    /// Framework identifier, surfaces in [`RunResult::framework`].
    fn framework(&self) -> &'static str;
    /// Execute the workload and return measured results.
    ///
    /// # Errors
    ///
    /// Returns an error string when the workload category is not
    /// supported by this runner, or when an internal precondition
    /// (e.g. workload params out of range) is violated.
    fn run(&self, workload: &Workload) -> Result<RunResult, String>;
}
