//! `bench-parity-runner-gpui` — execute the GPUI framework runner
//! against one workload and write the result JSON. Sibling to
//! `bench-parity-runner` in the `sigil-bench-parity` crate; lives here
//! so GPUI's heavy dependency tree does NOT enter the default harness
//! build.

#![allow(clippy::print_stdout, clippy::doc_markdown)]

use clap as _;
use data_table as _;
use gpui as _;
use rand as _;
use serde as _;
use serde_json as _;
use time as _;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bench_parity::{FrameworkRunner, RunResult, Workload};
use bench_parity_gpui::GpuiRunner;
use clap::Parser;
use time::OffsetDateTime;
use time::format_description::well_known::iso8601::{Config, Iso8601, TimePrecision};

const TIMESTAMP_FMT: u128 = Config::DEFAULT
    .set_time_precision(TimePrecision::Second {
        decimal_digits: None,
    })
    .encode();

#[derive(Parser, Debug)]
#[command(
    name = "bench-parity-runner-gpui",
    about = "Run the GPUI framework runner against one workload, write a RunResult JSON."
)]
struct Args {
    /// Framework name. Only `gpui` is accepted; the flag is required
    /// for symmetry with `bench-parity-runner`.
    #[arg(long)]
    framework: String,
    /// Path to the workload JSON file.
    #[arg(long)]
    workload: PathBuf,
    /// Output path for the RunResult JSON. Defaults to
    /// `crates/sigil-bench-parity/results/<framework>-<workload_id>-<timestamp>.json`.
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(out) => {
            println!("wrote {}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bench-parity-runner-gpui: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<PathBuf, String> {
    let args = Args::parse();
    let workload = load_workload(&args.workload)?;
    let result = dispatch(&args.framework, &workload)?;
    let output_path = resolve_output_path(args.output.as_deref(), &args.framework, &workload)?;
    write_result(&output_path, &result)?;
    Ok(output_path)
}

fn load_workload(path: &Path) -> Result<Workload, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("read workload {}: {e}", path.display()))?;
    serde_json::from_str::<Workload>(&raw).map_err(|e| format!("parse workload: {e}"))
}

fn dispatch(framework: &str, workload: &Workload) -> Result<RunResult, String> {
    match framework {
        "gpui" => GpuiRunner::new().run(workload),
        other => Err(format!(
            "bench-parity-runner-gpui only supports framework=gpui (got {other:?}); \
             use bench-parity-runner for sigil"
        )),
    }
}

fn resolve_output_path(
    output: Option<&Path>,
    framework: &str,
    workload: &Workload,
) -> Result<PathBuf, String> {
    if let Some(p) = output {
        return Ok(p.to_path_buf());
    }
    let ts = OffsetDateTime::now_utc()
        .format(&Iso8601::<TIMESTAMP_FMT>)
        .map_err(|e| format!("timestamp format: {e}"))?
        .replace(':', "-");
    let default_dir = default_results_dir();
    Ok(default_dir.join(format!("{framework}-{}-{ts}.json", workload.workload_id)))
}

fn default_results_dir() -> PathBuf {
    PathBuf::from("crates/bench-parity/results")
}

fn write_result(path: &Path, result: &RunResult) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(result).map_err(|e| format!("serialise: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}
