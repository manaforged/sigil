//! `bench-parity-aggregator` — read every `RunResult` JSON in a results
//! directory, group by `workload_id`, render a markdown report.
//!
//! Comparative "X vs Y" prose is intentionally NOT emitted. The harness
//! is a measurement substrate; comparative claims belong in long-form
//! analysis docs that cite the per-framework rows below.

#![allow(
    clippy::print_stdout,
    clippy::doc_markdown,
    clippy::cast_precision_loss
)]
// The lib re-uses these workspace deps; the bin pulls them in
// transitively via `sigil-bench-parity` itself. Silence the
// unused-extern-crate lint.

use hal as _;
use kernel as _;
use layout as _;
use reactive as _;
use serde as _;
use serde_json as _;
use sigil as _;
use sysinfo as _;
use time as _;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bench_parity::{EquivalenceCheck, HostInfo, RunResult};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "bench-parity-aggregator",
    about = "Aggregate RunResult JSON files into a markdown report."
)]
struct Args {
    /// Directory containing `*.json` RunResult files.
    #[arg(long)]
    results_dir: PathBuf,
    /// Output markdown path.
    #[arg(long)]
    output: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(path) => {
            println!("wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bench-parity-aggregator: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<PathBuf, String> {
    let args = Args::parse();
    let results = load_results(&args.results_dir)?;
    if results.is_empty() {
        return Err(format!(
            "no RunResult JSON files found under {}",
            args.results_dir.display()
        ));
    }
    let report = render_report(&results);
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(&args.output, report).map_err(|e| format!("write report: {e}"))?;
    Ok(args.output)
}

fn load_results(dir: &Path) -> Result<Vec<RunResult>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    let mut out: Vec<RunResult> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let parsed = serde_json::from_str::<RunResult>(&raw)
            .map_err(|e| format!("parse {}: {e}", path.display()))?;
        out.push(parsed);
    }
    Ok(out)
}

fn render_preamble(buf: &mut String) {
    buf.push_str("# Bench-parity report\n\n");
    buf.push_str(
        "Per-framework measurements on locked workloads. \
         Every \"X vs Y\" comparison must verify that BOTH frameworks \
         report `pass` for every equivalence check on the same workload \
         before any speedup claim is made.\n\n",
    );
    buf.push_str(
        "> **Methodology note.** Different runners may currently time \
         different surfaces — e.g. the Sigil runner times pure layout \
         (`LayoutEngine::compute`); the GPUI runner times \
         `VisualTestContext::draw` which includes paint-attempt setup \
         and a `catch_unwind` of the headless-paint panic. Until those \
         boundaries are reconciled, treat the rows below as \
         single-framework measurements on a shared workload, not as a \
         clean side-by-side comparison. See \
         `crates/sigil-bench-parity/README.md` § \"GPUI runner — \
         shipped, with a methodology caveat\".\n\n",
    );
}

fn render_report(results: &[RunResult]) -> String {
    let mut buf = String::new();
    render_preamble(&mut buf);

    if let Some(first) = results.first() {
        render_host_section(&first.host, &mut buf);
    }

    let mut by_workload: BTreeMap<&str, Vec<&RunResult>> = BTreeMap::new();
    for r in results {
        by_workload
            .entry(r.workload_id.as_str())
            .or_default()
            .push(r);
    }

    for (workload_id, rows) in by_workload {
        let _ = writeln!(buf, "## Workload — `{workload_id}`\n");
        buf.push_str(
            "| framework | iterations | median (µs) | p95 (µs) | p99 (µs) | equivalence |\n",
        );
        buf.push_str("|---|---:|---:|---:|---:|---|\n");
        for r in rows {
            let s = &r.summary;
            let _ = writeln!(
                buf,
                "| {} | {} | {:.2} | {:.2} | {:.2} | {} |",
                r.framework,
                s.iterations,
                s.median_us,
                s.p95_us,
                s.p99_us,
                render_equivalence(&r.equivalence_check_results),
            );
        }
        buf.push('\n');
    }
    buf
}

fn render_host_section(host: &HostInfo, buf: &mut String) {
    buf.push_str("## Hardware\n\n");
    let _ = writeln!(buf, "- OS: {}", host.os);
    let _ = writeln!(buf, "- CPU: {} ({} cores)", host.cpu, host.cpu_cores);
    let _ = writeln!(
        buf,
        "- Memory: {:.2} GiB",
        (host.total_memory_bytes as f64) / (1024.0 * 1024.0 * 1024.0),
    );
    let _ = writeln!(buf, "- rustc: {}", host.rustc_version);
    buf.push('\n');
}

fn render_equivalence(checks: &std::collections::HashMap<String, EquivalenceCheck>) -> String {
    if checks.is_empty() {
        return "(none declared)".into();
    }
    let mut parts: Vec<String> = checks
        .iter()
        .map(|(name, verdict)| {
            let tag = match verdict {
                EquivalenceCheck::Pass => "pass",
                EquivalenceCheck::Fail { .. } => "fail",
                EquivalenceCheck::NotApplicable => "n/a",
            };
            format!("{name}={tag}")
        })
        .collect();
    parts.sort();
    parts.join(", ")
}
