use std::path::Path;
use std::process::Command;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sysinfo::System;

const SCHEMA: &str = "record-v1";
pub const BUDGET_60_NS: u64 = 16_667_000;
pub const BUDGET_120_NS: u64 = 8_333_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub framework: String,
    pub framework_version: String,
    pub harness_commit: String,
    pub scenario: String,
    pub display_mode: String,
    pub backend: String,
    pub viewport: (f32, f32),
    pub host: Host,
    pub started: String,
    pub pulses: usize,
    pub interval_ns: Vec<u64>,
    pub summary: Summary,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Host {
    pub os: String,
    pub cpu: String,
    pub cores: usize,
    pub memory_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub p50_ns: u64,
    pub p90_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub max_ns: u64,
    pub over_60hz: usize,
    pub over_120hz: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

impl Check {
    #[must_use]
    pub fn new(name: &str, pass: bool, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pass,
            detail: detail.into(),
        }
    }
}

impl Record {
    #[must_use]
    pub fn new(framework: &str, version: &str, backend: &str, viewport: (f32, f32)) -> Self {
        Self {
            schema: SCHEMA.into(),
            framework: framework.into(),
            framework_version: version.into(),
            harness_commit: git_head(),
            scenario: String::new(),
            display_mode: "windowed".into(),
            backend: backend.into(),
            viewport,
            host: Host::capture(),
            started: now_utc(),
            pulses: 0,
            interval_ns: Vec::new(),
            summary: Summary::default(),
            checks: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn finish(&mut self, scenario: &str, marks: &[Instant]) {
        self.scenario = scenario.into();
        self.pulses = marks.len();
        self.interval_ns = marks
            .iter()
            .zip(marks.iter().skip(1))
            .map(|(a, b)| u64::try_from((*b - *a).as_nanos()).unwrap_or(u64::MAX))
            .collect();
        self.summary = summarize(&self.interval_ns);
    }

    #[must_use]
    pub fn all_pass(&self) -> bool {
        self.checks.iter().all(|c| c.pass)
    }

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn read(path: &Path) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let record: Self = serde_json::from_str(&raw).map_err(std::io::Error::other)?;
        if record.schema != SCHEMA {
            return Err(std::io::Error::other(format!(
                "{}: schema {} is not {SCHEMA}",
                path.display(),
                record.schema
            )));
        }
        Ok(record)
    }
}

impl Host {
    #[must_use]
    pub fn capture() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Self {
            os: format!(
                "{} {}",
                System::name().unwrap_or_else(|| "unknown".into()),
                System::os_version().unwrap_or_default()
            )
            .trim()
            .into(),
            cpu: sys
                .cpus()
                .first()
                .map_or_else(|| "unknown".into(), |c| c.brand().into()),
            cores: sys.cpus().len(),
            memory_bytes: sys.total_memory(),
        }
    }
}

#[must_use]
pub fn summarize(intervals: &[u64]) -> Summary {
    if intervals.is_empty() {
        return Summary::default();
    }
    let mut sorted = intervals.to_vec();
    sorted.sort_unstable();
    let at = |pct: usize| {
        let ix = (sorted.len() * pct).div_ceil(100).saturating_sub(1);
        sorted.get(ix).copied().unwrap_or(0)
    };
    Summary {
        p50_ns: at(50),
        p90_ns: at(90),
        p95_ns: at(95),
        p99_ns: at(99),
        max_ns: sorted.last().copied().unwrap_or(0),
        over_60hz: intervals.iter().filter(|&&ns| ns > BUDGET_60_NS).count(),
        over_120hz: intervals.iter().filter(|&&ns| ns > BUDGET_120_NS).count(),
    }
}

fn git_head() -> String {
    let out = Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().into(),
        _ => "unknown".into(),
    }
}

fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("{secs}")
}
