//! `bench-parity-runner` — execute one framework against one workload
//! and write the result JSON.

#![allow(clippy::print_stdout, clippy::doc_markdown)]

// The lib re-uses these workspace deps; the bin pulls them in transitively
// via `sigil-bench-parity` itself. Silence the unused-extern-crate lint.
use serde as _;
use sigil as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sysinfo as _;

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use bench_parity::{FrameworkRunner, RunResult, SigilRunner, Workload};
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
    name = "bench-parity-runner",
    about = "Run one framework against one workload, write a RunResult JSON."
)]
struct Args {
    /// Framework name. Currently supported: `sigil`.
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
            eprintln!("bench-parity-runner: {e}");
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

fn load_workload(path: &std::path::Path) -> Result<Workload, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("read workload {}: {e}", path.display()))?;
    serde_json::from_str::<Workload>(&raw).map_err(|e| format!("parse workload: {e}"))
}

fn dispatch(framework: &str, workload: &Workload) -> Result<RunResult, String> {
    match framework {
        "sigil" => SigilRunner::new().run(workload),
        other => Err(format!(
            "unknown framework {other:?}; supported: sigil. \
             GPUI / TanStack runners land in follow-up dispatches."
        )),
    }
}

fn resolve_output_path(
    output: Option<&std::path::Path>,
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
    PathBuf::from("crates/sigil-bench-parity/results")
}

fn write_result(path: &std::path::Path, result: &RunResult) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(result).map_err(|e| format!("serialise: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}
