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

use data_table as _;
use serde as _;
use serde_json as _;
use sigil as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_text as _;
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
         A numeric `vs gpui` cell requires matching source, output, \
         reuse/tick, host, and verdicts. A numeric `vs frame` cell is retained compute \
         versus reconstruct — not the same mode.\n\n",
    );
    buf.push_str(
        "Host setup is untimed. `vs gpui` is a number only when both \
         sides stamp the same reuse/tick mode, the same host \
         (OS/CPU/rustc), the same verdicts (Pass vs N/A is not a pair), \
         the same clean source revision, the same canonical output, \
         and both CVs stay at or under 0.25. `vs frame` is a number \
         when both sides pass `ratio_ok`, share that host, both CVs \
         stay at or under 0.25, and reuse is complementary (1 vs 0) \
         with the same tick flag. Do not quote `vs frame` as 1:1 \
         remount. Krausest rows carry id/label/remove text. Tick \
         requires last_done_did_move. An empty check map is not a \
         pass. VOID if construct recorded all layout hits. NOISY if \
         either CV is above the ceiling. `— source` means missing, dirty, \
         or different source. `— output` means the canonical output is \
         missing or different.\n\n",
    );
}

fn render_report(results: &[RunResult]) -> String {
    let mut buf = String::new();
    render_preamble(&mut buf);

    if let Some(first) = results.first() {
        render_host_section(&first.host, results, &mut buf);
    }

    let mut by_workload: BTreeMap<&str, Vec<&RunResult>> = BTreeMap::new();
    for r in results {
        by_workload
            .entry(r.workload_id.as_str())
            .or_default()
            .push(r);
    }

    for (workload_id, rows) in by_workload {
        render_workload_table(workload_id, &rows, &mut buf);
    }
    buf
}
/// One workload section: header row + one line per framework result.
fn render_workload_table(workload_id: &str, rows: &[&RunResult], buf: &mut String) {
    let _ = writeln!(buf, "## Workload — `{workload_id}`\n");
    buf.push_str(
            "| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |\n",
        );
    buf.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    let gpui = rows.iter().find(|r| r.framework == "gpui").copied();
    let sigil = rows.iter().find(|r| r.framework == "sigil").copied();
    for r in rows {
        let s = &r.summary;
        let other = if r.framework == "gpui" { sigil } else { gpui };
        let vs = bench_parity::vs_gpui(r, other);
        let frame = bench_parity::vs_frame(r, other);
        let rss = if r.host.process_rss_bytes == 0 {
            "—".into()
        } else {
            format!(
                "{:.1}",
                (r.host.process_rss_bytes as f64) / (1024.0 * 1024.0)
            )
        };
        let _ = writeln!(
            buf,
            "| {} | {} | {:.2} | {:.2} | {:.3} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            r.framework,
            s.iterations,
            s.median_us,
            s.p95_us,
            s.cv,
            r.verify.get("layout_hits").copied().unwrap_or(0),
            r.verify.get("layout_misses").copied().unwrap_or(0),
            r.verify.get("first_visible").copied().unwrap_or(0),
            r.verify.get("visible_last").copied().unwrap_or(0),
            r.verify.get("book_rows").copied().unwrap_or(0),
            rss,
            vs,
            frame,
            render_equivalence(&r.equivalence_check_results),
        );
    }
}

fn render_host_section(host: &HostInfo, results: &[RunResult], buf: &mut String) {
    buf.push_str("## Hardware\n\n");
    let _ = writeln!(buf, "- OS: {}", host.os);
    let _ = writeln!(buf, "- CPU: {} ({} cores)", host.cpu, host.cpu_cores);
    let _ = writeln!(
        buf,
        "- Memory: {:.2} GiB",
        (host.total_memory_bytes as f64) / (1024.0 * 1024.0 * 1024.0),
    );
    let _ = writeln!(buf, "- rustc: {}", host.rustc_version);
    let mixed = results.iter().any(|r| {
        r.host.os != host.os || r.host.cpu != host.cpu || r.host.rustc_version != host.rustc_version
    });
    if mixed {
        buf.push_str("- Host mix: results disagree on OS/CPU/rustc. No `vs gpui` cell is valid.\n");
    }
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
