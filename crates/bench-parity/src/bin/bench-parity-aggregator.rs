#![allow(
    clippy::print_stdout,
    clippy::doc_markdown,
    clippy::cast_precision_loss
)]

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
    #[arg(long)]
    results_dir: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    one_to_one: bool,
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
    let mut results = load_results(&args.results_dir)?;
    if args.one_to_one {
        results.retain(bench_parity::is_construct);
    }
    if results.is_empty() {
        return Err(format!(
            "no RunResult JSON files found under {}",
            args.results_dir.display()
        ));
    }
    let report = render_report(&results, args.one_to_one);
    if args.one_to_one {
        reject_unpaired(&results)?;
    }
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

fn render_preamble(buf: &mut String, one_to_one: bool) {
    buf.push_str("# Bench-parity report\n\n");
    if one_to_one {
        buf.push_str(
            "1:1 construct versus crates.io GPUI 0.2.2. Both sides build \
             the tree and layout it every sample. No retain, no tick. \
             Quote only a numeric `vs gpui` cell. There is no `vs frame` \
             column.\n\n",
        );
    } else {
        buf.push_str(
            "Per-framework measurements on locked workloads. \
         A numeric `vs gpui` cell requires matching source, output, \
         reuse/tick, host, and verdicts. A numeric `vs frame` cell is retained compute \
         versus reconstruct — not the same mode.\n\n",
        );
    }
    if one_to_one {
        buf.push_str(
            "Host setup is untimed. A cell prints only when `one_to_one` \
             holds: construct on both sides, same host, same clean \
             revision, same mounted row count, same canonical output, \
             both CVs at or under 0.25, layout and text actually ran. \
             VOID / NOISY / — source / — visible / — output / — checks \
             fail this report.\n\n",
        );
    } else {
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
}

fn render_report(results: &[RunResult], one_to_one: bool) -> String {
    let mut buf = String::new();
    render_preamble(&mut buf, one_to_one);

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
        render_workload_table(workload_id, &rows, one_to_one, &mut buf);
    }
    buf
}

fn reject_unpaired(results: &[RunResult]) -> Result<(), String> {
    let mut by_workload: BTreeMap<&str, Vec<&RunResult>> = BTreeMap::new();
    for r in results {
        by_workload
            .entry(r.workload_id.as_str())
            .or_default()
            .push(r);
    }
    let mut failed = Vec::new();
    for (id, rows) in by_workload {
        let gpui = rows.iter().find(|r| r.framework == "gpui").copied();
        let sigil = rows.iter().find(|r| r.framework == "sigil").copied();
        let Some(sigil) = sigil else {
            failed.push(format!("{id}: missing sigil row"));
            continue;
        };
        let Some(gpui) = gpui else {
            failed.push(format!("{id}: missing gpui row"));
            continue;
        };
        if !bench_parity::one_to_one(sigil, gpui) {
            failed.push(format!(
                "{id}: not 1:1 (vs gpui = {})",
                bench_parity::vs_gpui(sigil, Some(gpui))
            ));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("; "))
    }
}

fn render_workload_table(
    workload_id: &str,
    rows: &[&RunResult],
    one_to_one: bool,
    buf: &mut String,
) {
    let _ = writeln!(buf, "## Workload — `{workload_id}`\n");
    if one_to_one {
        buf.push_str(
            "| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |\n",
        );
        buf.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    } else {
        buf.push_str(
            "| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |\n",
        );
        buf.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    }
    render_result_rows(buf, rows, one_to_one);
}

fn render_result_rows(buf: &mut String, rows: &[&RunResult], one_to_one: bool) {
    let gpui = rows.iter().find(|r| r.framework == "gpui").copied();
    let sigil = rows.iter().find(|r| r.framework == "sigil").copied();
    for r in rows {
        let s = &r.summary;
        let other = if r.framework == "gpui" { sigil } else { gpui };
        let vs = bench_parity::vs_gpui(r, other);
        let rss = if r.host.process_rss_bytes == 0 {
            "—".into()
        } else {
            format!(
                "{:.1}",
                (r.host.process_rss_bytes as f64) / (1024.0 * 1024.0)
            )
        };
        let eq = render_equivalence(&r.equivalence_check_results);
        let hits = r.verify.get("layout_hits").copied().unwrap_or(0);
        let misses = r.verify.get("layout_misses").copied().unwrap_or(0);
        let first = r.verify.get("first_visible").copied().unwrap_or(0);
        let visible = r.verify.get("visible_last").copied().unwrap_or(0);
        let book = r.verify.get("book_rows").copied().unwrap_or(0);
        let cells = format!(
            "| {} | {} | {:.2} | {:.2} | {:.3} | {} | {} | {} | {} | {} | {} | {} |",
            r.framework,
            s.iterations,
            s.median_us,
            s.p95_us,
            s.cv,
            hits,
            misses,
            first,
            visible,
            book,
            rss,
            vs,
        );
        if one_to_one {
            let _ = writeln!(buf, "{cells} {eq} |");
        } else {
            let frame = bench_parity::vs_frame(r, other);
            let _ = writeln!(buf, "{cells} {frame} | {eq} |");
        }
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
    buf.push_str("- GPUI: crates.io 0.2.2\n");
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
