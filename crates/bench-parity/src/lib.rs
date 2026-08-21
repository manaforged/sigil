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

mod compare;
mod proof;
pub mod runners;

pub use compare::{
    CV_CEILING, comparable, cv_ok, frame_pair, layout_void, ratio_ok, same_checks, same_host,
    same_mode, same_output, same_source, same_verdicts, vs_frame, vs_gpui,
};
pub use proof::{OutputProof, SourceInfo, milli_px};
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
    /// Timed surface: `layout` (default) or `layout_paint`.
    #[serde(default)]
    pub surface: String,
    /// Keep the host tree and only change scroll / inputs between samples.
    #[serde(default)]
    pub reuse: bool,
    /// Cells per row for `flex-tree` and `data-table` (0 → 8).
    #[serde(default)]
    pub cells_per_row: usize,
}

/// Timed surface for a workload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    /// Layout only (GPUI `layout_as_root` + `prepaint`; Sigil `compute`).
    Layout,
    /// Layout plus CPU scene (GPUI paint when it does not panic; Sigil emit).
    LayoutPaint,
}

impl WorkloadParams {
    /// Resolved surface. Empty string is [`Surface::Layout`].
    #[must_use]
    pub fn surface(&self) -> Surface {
        match self.surface.as_str() {
            "layout_paint" => Surface::LayoutPaint,
            _ => Surface::Layout,
        }
    }

    /// Cells per row, default 8. `gpui-24-field` is locked to 24.
    #[must_use]
    pub fn cells(&self) -> usize {
        if self.gpui_table() {
            data_table::COLUMN_COUNT
        } else if self.cells_per_row == 0 {
            8
        } else {
            self.cells_per_row
        }
    }

    /// True when this workload is the live GPUI `data_table` clone.
    #[must_use]
    pub fn gpui_table(&self) -> bool {
        self.row_shape == "gpui-24-field"
    }

    /// True when samples mutate visible quotes in place (no scroll).
    #[must_use]
    pub fn tick(&self) -> bool {
        self.scroll_pattern.starts_with("tick")
    }

    /// True when the window sits on the last page of the book.
    #[must_use]
    pub fn tail(&self) -> bool {
        self.scroll_pattern.contains("tail")
    }

    /// Row height. The 24-column clone uses the live window constant.
    #[must_use]
    pub fn row_h(&self) -> f32 {
        if self.gpui_table() {
            data_table::ROW_HEIGHT
        } else {
            self.row_height_px
        }
    }

    /// Viewport size. The 24-column clone uses the live window pane.
    #[must_use]
    pub fn viewport(&self) -> (f32, f32) {
        if self.gpui_table() {
            (data_table::TABLE_WIDTH, data_table::VIEWPORT_HEIGHT)
        } else {
            (self.viewport_width_px, self.viewport_height_px)
        }
    }
}

/// Krausest 4-cell row — same numbers and strings on both runners.
pub const ID_CELL_W: f32 = 80.0;
/// Label column width.
pub const LABEL_CELL_W: f32 = 320.0;
/// Action column width.
pub const ACTION_CELL_W: f32 = 80.0;
/// Trailing spacer width.
pub const SPACER_CELL_W: f32 = 800.0;
/// Sum of the four krausest cells.
pub const ROW_WIDTH: f32 = ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W;
/// Data-table cell width (GPUI `data_table` example uses fixed columns).
pub const TABLE_CELL_W: f32 = 100.0;
/// Text cells in a krausest row (id, label, remove). Spacer is empty.
pub const KRAUSEST_TEXT_CELLS: usize = 3;
/// Remove-column label. Same bytes on both runners.
pub const KRAUSEST_ACTION: &str = "remove";

/// Krausest id cell. Same string on both runners.
#[must_use]
pub fn krausest_id(index: u32) -> String {
    index.to_string()
}

/// Krausest label cell. Same string on both runners.
#[must_use]
pub fn krausest_label(index: u32) -> String {
    format!("Item #{index}")
}

/// Pass when the runner held `want` source rows, not only the window.
#[must_use]
pub fn book_held(got: usize, want: usize) -> EquivalenceCheck {
    if got == want && want > 0 {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: format!("book {got} != {want}"),
        }
    }
}

/// Pass when the visible band reaches the last source row.
#[must_use]
pub fn tail_window(first: usize, visible: usize, rows: usize) -> EquivalenceCheck {
    let last = first.saturating_add(visible);
    let min_first = rows.saturating_sub(visible.saturating_add(1));
    if rows > 0 && visible > 0 && last >= rows && first >= min_first {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: format!("first {first} visible {visible} rows {rows}"),
        }
    }
}

/// Inclusive visible-row band both runners accept.
///
/// GPUI `uniform_list` mounts `ceil(viewport / row)`. Sigil mounts that
/// page plus one page of overscan on each side (two pages at the top or
/// tail, three mid-list). A count in `[page, 3*page+1]` is the same check.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn visible_band(viewport_h: f32, row_h: f32, row_count: usize) -> (usize, usize) {
    if row_count == 0 || row_h <= 0.0 {
        return (0, 0);
    }
    let page = (viewport_h / row_h).ceil() as usize;
    let lo = page.min(row_count);
    let hi = page.saturating_mul(3).saturating_add(1).min(row_count);
    (lo, hi)
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
    /// Proof the timed loop did real work: hits, misses, measures, visible rows.
    #[serde(default)]
    pub verify: HashMap<String, u64>,
    /// Canonical output from an untimed verification frame. Ratios require an
    /// exact match, so older results without this field fail closed.
    #[serde(default)]
    pub output: Option<OutputProof>,
    /// Source revision used for the run. Ratios require the same clean tree.
    #[serde(default)]
    pub source: Option<SourceInfo>,
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
    /// Coefficient of variation (`stdev / mean`). Zero when mean is 0.
    #[serde(default)]
    pub cv: f64,
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
                cv: 0.0,
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
        let mean_ns = (sum as f64) / (n as f64);
        let var = sorted
            .iter()
            .map(|&v| {
                let d = v as f64 - mean_ns;
                d * d
            })
            .sum::<f64>()
            / (n as f64);
        let cv = if mean_ns > 0.0 {
            var.sqrt() / mean_ns
        } else {
            0.0
        };
        Self {
            iterations: n,
            median_us: pct(0.50),
            p95_us: pct(0.95),
            p99_us: pct(0.99),
            mean_us: mean_ns / 1_000.0,
            cv,
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
    /// Resident set of this process after the run, in bytes. `0` on
    /// older JSON that omitted the field.
    #[serde(default)]
    pub process_rss_bytes: u64,
}

impl HostInfo {
    /// Probe the running host. Uses `sysinfo` for CPU+OS+memory.
    #[must_use]
    pub fn capture() -> Self {
        use sysinfo::{Pid, ProcessesToUpdate, System};
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let pid = Pid::from_u32(std::process::id());
        sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        let process_rss_bytes = sys.process(pid).map_or(0, sysinfo::Process::memory);
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
            process_rss_bytes,
        }
    }
}

fn rustc_version_string() -> String {
    proof::command_line("rustc", &["--version"])
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

#[cfg(test)]
#[path = "lib_band_tests.rs"]
mod band_tests;
#[cfg(test)]
#[path = "lib_book_tests.rs"]
mod book_tests;
