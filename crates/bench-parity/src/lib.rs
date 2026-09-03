#![allow(clippy::module_name_repetitions)]

use clap as _;
use serde_json as _;
use time as _;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

mod compare;
mod construct;
mod proof;
pub mod runners;

pub use compare::{
    CV_CEILING, comparable, cv_ok, frame_pair, is_construct, layout_void, one_to_one, ratio_ok,
    same_checks, same_host, same_mode, same_output, same_source, same_verdicts, same_visible,
    vs_frame, vs_gpui,
};
pub use construct::{
    NESTED_LEAF, construct_proof, expected_texts, form_key, form_value, overlay_label,
    text_construct, text_construct_id, wrap_paragraph,
};
pub use proof::{OutputProof, SourceInfo, milli_px};
pub use runners::SigilRunner;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workload {
    pub workload_id: String,
    pub description: String,
    pub category: String,
    pub params: WorkloadParams,
    #[serde(default)]
    pub equivalence_checks: Vec<EquivalenceCheckSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadParams {
    #[serde(default)]
    pub row_count: usize,
    #[serde(default)]
    pub row_height_px: f32,
    #[serde(default)]
    pub viewport_width_px: f32,
    #[serde(default)]
    pub viewport_height_px: f32,
    #[serde(default)]
    pub row_shape: String,
    #[serde(default)]
    pub scroll_pattern: String,
    #[serde(default)]
    pub iterations: usize,
    #[serde(default)]
    pub warmup: usize,
    #[serde(default)]
    pub surface: String,
    #[serde(default)]
    pub reuse: bool,
    #[serde(default)]
    pub cells_per_row: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    Layout,
    LayoutPaint,
}

impl WorkloadParams {
    #[must_use]
    pub fn surface(&self) -> Surface {
        match self.surface.as_str() {
            "layout_paint" => Surface::LayoutPaint,
            _ => Surface::Layout,
        }
    }

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

    #[must_use]
    pub fn gpui_table(&self) -> bool {
        self.row_shape == "gpui-24-field"
    }

    #[must_use]
    pub fn tick(&self) -> bool {
        self.scroll_pattern.starts_with("tick")
    }

    #[must_use]
    pub fn tail(&self) -> bool {
        self.scroll_pattern.contains("tail")
    }

    #[must_use]
    pub fn row_h(&self) -> f32 {
        if self.gpui_table() {
            data_table::ROW_HEIGHT
        } else {
            self.row_height_px
        }
    }

    #[must_use]
    pub fn viewport(&self) -> (f32, f32) {
        if self.gpui_table() {
            (data_table::TABLE_WIDTH, data_table::VIEWPORT_HEIGHT)
        } else {
            (self.viewport_width_px, self.viewport_height_px)
        }
    }
}

pub const ID_CELL_W: f32 = 80.0;
pub const LABEL_CELL_W: f32 = 320.0;
pub const ACTION_CELL_W: f32 = 80.0;
pub const SPACER_CELL_W: f32 = 800.0;
pub const ROW_WIDTH: f32 = ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W;
pub const TABLE_CELL_W: f32 = 100.0;
pub const KRAUSEST_TEXT_CELLS: usize = 3;
pub const KRAUSEST_ACTION: &str = "remove";

#[must_use]
pub fn krausest_id(index: u32) -> String {
    index.to_string()
}

#[must_use]
pub fn krausest_label(index: u32) -> String {
    format!("Item #{index}")
}

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

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "values are bounded to the target range before the cast"
)]
pub fn visible_page(viewport_h: f32, row_h: f32, row_count: usize) -> usize {
    if row_count == 0 || row_h <= 0.0 {
        return 0;
    }
    ((viewport_h / row_h).ceil() as usize).max(1).min(row_count)
}

#[must_use]
pub fn visible_band(viewport_h: f32, row_h: f32, row_count: usize) -> (usize, usize) {
    let page = visible_page(viewport_h, row_h, row_count);
    let hi = page.saturating_mul(3).saturating_add(1).min(row_count);
    (page, hi)
}

#[must_use]
pub fn visible_span(params: &WorkloadParams) -> (usize, usize) {
    let (_, vh) = params.viewport();
    let page = visible_page(vh, params.row_h(), params.row_count);
    let block = (page / 4).max(1);
    if !params.reuse && !params.tick() {
        let hi = page
            .saturating_add(block + 1)
            .min(params.row_count.max(page));
        (page, hi.max(page))
    } else {
        let (lo, hi) = visible_band(vh, params.row_h(), params.row_count);
        (
            lo,
            hi.saturating_add(2 * block).min(params.row_count.max(hi)),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceCheckSpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EquivalenceCheck {
    Pass,
    Fail { detail: String },
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub framework: String,
    pub workload_id: String,
    pub samples_ns: Vec<u64>,
    pub summary: RunSummary,
    pub equivalence_check_results: HashMap<String, EquivalenceCheck>,
    pub host: HostInfo,
    #[serde(default)]
    pub verify: HashMap<String, u64>,
    #[serde(default)]
    pub output: Option<OutputProof>,
    #[serde(default)]
    pub source: Option<SourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub iterations: usize,
    pub median_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub mean_us: f64,
    #[serde(default)]
    pub cv: f64,
}

impl RunSummary {
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub os: String,
    pub cpu: String,
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub rustc_version: String,
    #[serde(default)]
    pub process_rss_bytes: u64,
}

impl HostInfo {
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

pub trait FrameworkRunner {
    fn framework(&self) -> &'static str;
    fn run(&self, workload: &Workload) -> Result<RunResult, String>;
}

#[cfg(test)]
#[path = "lib_band_tests.rs"]
mod band_tests;
#[cfg(test)]
#[path = "lib_book_tests.rs"]
mod book_tests;
