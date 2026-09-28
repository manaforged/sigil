#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

#[path = "perf_gate/benches.rs"]
mod benches;
#[path = "perf_gate/budgets.rs"]
mod budgets;
#[path = "perf_gate/drive.rs"]
mod drive;
#[path = "events_support/input.rs"]
mod input;
#[path = "perf_gate/rigs.rs"]
mod rigs;

use std::process::ExitCode;

fn percentile(sorted: &[u64], pct: f64) -> f64 {
    let index = ((sorted.len().saturating_sub(1)) as f64 * pct).round() as usize;
    sorted
        .get(index)
        .map_or(0.0, |&nanos| nanos as f64 / 1000.0)
}

fn frames(table: &budgets::Budgets, only: Option<&str>) -> bool {
    let mut ok = true;
    println!("scenario  allocs/frame(max)  budget  bytes/frame  p50us  p95us  p99us");
    for (scenario, budget) in &table.frames {
        let name = format!("{scenario:?}").to_lowercase();
        if only.is_some_and(|only| only != name) {
            continue;
        }
        let mut sample = drive::run(*scenario);
        sample.nanos.sort_unstable();
        let pass = sample.max_allocs <= budget.allocs;
        ok &= pass;
        println!(
            "{:<9} {:>17}  {:>6}  {:>11}  {:>5.1}  {:>5.1}  {:>5.1}  {}",
            name,
            sample.max_allocs,
            budget.allocs,
            sample.mean_bytes,
            percentile(&sample.nanos, 0.50),
            percentile(&sample.nanos, 0.95),
            percentile(&sample.nanos, 0.99),
            if pass { "ok" } else { "OVER BUDGET" }
        );
    }
    ok
}

fn main() -> ExitCode {
    let table = budgets::load();
    let args: Vec<String> = std::env::args().collect();
    let ok = args.iter().position(|arg| arg == "--benches").map_or_else(
        || {
            frames(
                &table,
                args.iter()
                    .skip(1)
                    .find(|arg| !arg.starts_with('-'))
                    .map(String::as_str),
            )
        },
        |at| benches::check(&table, args.get(at + 1).map(String::as_str)),
    );
    if ok {
        ExitCode::SUCCESS
    } else {
        eprintln!("perf gate failed; budgets live in crates/sigil/perf_budgets.toml");
        ExitCode::FAILURE
    }
}
