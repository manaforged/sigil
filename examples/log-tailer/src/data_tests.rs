#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn generate_emits_requested_count() {
    let lines = generate(10);
    assert_eq!(lines.len(), 10);
}

#[test]
fn level_cycles_through_all_five() {
    let lines = generate(5);
    let labels: Vec<_> = lines.iter().map(|line| line.level).collect();
    assert_eq!(
        labels,
        vec![
            Level::Trace,
            Level::Debug,
            Level::Info,
            Level::Warn,
            Level::Error,
        ]
    );
}

#[test]
fn format_has_level_label() {
    let lines = generate(3);
    assert!(lines[2].format().contains("INFO"));
}
