#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn percentiles_come_from_the_sorted_intervals() {
    let intervals: Vec<u64> = (1..=100).map(|n| n * 1_000_000).collect();
    let s = summarize(&intervals);
    assert_eq!(s.p50_ns, 50_000_000);
    assert_eq!(s.p95_ns, 95_000_000);
    assert_eq!(s.p99_ns, 99_000_000);
    assert_eq!(s.max_ns, 100_000_000);
    assert_eq!(s.over_60hz, 84);
    assert_eq!(s.over_120hz, 92);
    assert_eq!(summarize(&[]), Summary::default());
}

#[test]
fn a_record_round_trips_through_json() {
    let dir = std::env::temp_dir().join(format!("bench-spec-{}", std::process::id()));
    let path = dir.join("r.json");
    let mut record = Record::new("test", "0", "none", (1.0, 2.0));
    let base = std::time::Instant::now();
    let marks: Vec<_> = (0..4)
        .map(|i| base + std::time::Duration::from_millis(16 * i))
        .collect();
    record.finish("s", &marks);
    record.checks.push(Check::new("ok", true, ""));
    record.write(&path).unwrap();
    let back = Record::read(&path).unwrap();
    assert_eq!(back.pulses, 4);
    assert_eq!(back.interval_ns.len(), 3);
    assert!(back.all_pass());
    std::fs::remove_dir_all(dir).unwrap();
}
