use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Framework-neutral output from an untimed verification frame.
///
/// `values` contains workload-specific counts such as rows and cells.
/// `text_rows` contains the actual visible strings in row order. Geometry is
/// quantized to one thousandth of a logical pixel before serialization.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutputProof {
    /// Versioned probe shape, for example `visible-rows-v1`.
    pub schema: String,
    /// Root width and height in thousandths of a logical pixel.
    pub root_size_milli_px: [i64; 2],
    /// Workload-specific structural counts.
    #[serde(default)]
    pub values: BTreeMap<String, u64>,
    /// Visible text grouped by row in document order.
    #[serde(default)]
    pub text_rows: Vec<Vec<String>>,
}

/// Source identity captured when a runner executes.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Git commit, or `unknown` outside a Git checkout.
    pub revision: String,
    /// True when tracked or untracked files differ from the commit.
    pub dirty: bool,
    /// Harness crate version.
    pub harness_version: String,
}

impl SourceInfo {
    /// Capture the current repository revision and worktree status.
    #[must_use]
    pub fn capture() -> Self {
        Self {
            revision: command_line("git", &["rev-parse", "HEAD"]),
            dirty: !command_line("git", &["status", "--porcelain"]).is_empty(),
            harness_version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

/// Quantize a logical-pixel value for stable cross-framework comparison.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn milli_px(value: f32) -> i64 {
    f64::from(value).mul_add(1_000.0, 0.0).round() as i64
}

pub(crate) fn command_line(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |stdout| stdout.trim().to_string())
}
