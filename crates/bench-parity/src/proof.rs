use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutputProof {
    pub schema: String,
    pub root_size_milli_px: [i64; 2],
    #[serde(default)]
    pub values: BTreeMap<String, u64>,
    #[serde(default)]
    pub text_rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub revision: String,
    pub dirty: bool,
    pub harness_version: String,
}

impl SourceInfo {
    #[must_use]
    pub fn capture() -> Self {
        Self {
            revision: command_line("git", &["rev-parse", "HEAD"]),
            dirty: worktree_dirty(),
            harness_version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
pub fn milli_px(value: f32) -> i64 {
    f64::from(value).mul_add(1_000.0, 0.0).round() as i64
}

fn worktree_dirty() -> bool {
    !command_line(
        "git",
        &[
            "status",
            "--porcelain",
            "--",
            ".",
            ":!crates/bench-parity/results",
        ],
    )
    .is_empty()
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
