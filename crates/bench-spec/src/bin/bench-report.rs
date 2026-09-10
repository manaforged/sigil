#![allow(clippy::print_stdout)]

use data_table as _;
use serde as _;
use serde_json as _;
use sysinfo as _;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bench_spec::Record;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next().map(PathBuf::from) else {
        eprintln!("usage: bench-report <records-dir> [out.md]");
        return ExitCode::from(2);
    };
    let out = args.next().map(PathBuf::from);
    match run(&dir) {
        Ok(report) => {
            if let Some(path) = out {
                if let Err(e) = std::fs::write(&path, &report) {
                    eprintln!("bench-report: write {}: {e}", path.display());
                    return ExitCode::from(1);
                }
                println!("wrote {}", path.display());
            } else {
                print!("{report}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bench-report: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(dir: &Path) -> std::io::Result<String> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json") {
            records.push(Record::read(&path)?);
        }
    }
    if records.is_empty() {
        return Err(std::io::Error::other(format!(
            "no record-v1 json under {}",
            dir.display()
        )));
    }
    records.sort_by(|a, b| {
        (&a.scenario, &a.framework, &a.started).cmp(&(&b.scenario, &b.framework, &b.started))
    });
    Ok(render(&records))
}

fn render(records: &[Record]) -> String {
    let mut by_scenario: BTreeMap<&str, Vec<&Record>> = BTreeMap::new();
    for r in records {
        by_scenario.entry(&r.scenario).or_default().push(r);
    }
    let mut out = String::from("# Benchmark records\n\n");
    if let Some(host) = records.first().map(|r| &r.host) {
        writeln!(
            out,
            "Host: {} · {} · {} cores · {} GB\n",
            host.os,
            host.cpu,
            host.cores,
            host.memory_bytes / 1_000_000_000
        )
        .ok();
    }
    out.push_str("Intervals are between consecutive 16 ms driver pulses in a real window. ");
    out.push_str("Overruns count intervals above 16.67 ms (60 Hz) and 8.33 ms (120 Hz). ");
    out.push_str("A row with a failed check is not comparable.\n");
    for (scenario, rows) in by_scenario {
        writeln!(out, "\n## {scenario}\n").ok();
        out.push_str(
            "| framework | version | backend | commit | pulses | p50 ms | p90 ms | p95 ms | p99 ms | max ms | >60Hz | >120Hz | checks |\n",
        );
        out.push_str("| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n");
        for r in &rows {
            out.push_str(&row(r));
        }
        for r in rows {
            for note in &r.notes {
                writeln!(out, "\n- {}: {note}", r.framework).ok();
            }
        }
    }
    out
}

fn row(r: &Record) -> String {
    let s = r.summary;
    let checks = if r.all_pass() {
        "pass".to_string()
    } else {
        let failed: Vec<&str> = r
            .checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| c.name.as_str())
            .collect();
        format!("FAIL {}", failed.join(","))
    };
    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
        r.framework,
        r.framework_version,
        r.backend,
        r.harness_commit,
        r.pulses,
        ms(s.p50_ns),
        ms(s.p90_ns),
        ms(s.p95_ns),
        ms(s.p99_ns),
        ms(s.max_ns),
        s.over_60hz,
        s.over_120hz,
        checks
    )
}

fn ms(ns: u64) -> String {
    let whole = ns / 1_000_000;
    let frac = (ns % 1_000_000) / 10_000;
    format!("{whole}.{frac:02}")
}
