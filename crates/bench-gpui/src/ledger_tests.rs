#![allow(clippy::unwrap_used)]

use bench_spec::ledger::PULSES;

use super::Ledger;

#[test]
fn the_model_replays_to_the_expected_state() {
    let out = std::env::temp_dir().join(format!("ledger-gpui-{}.json", std::process::id()));
    let mut ledger = Ledger::model(2_000, out);
    for _ in 0..PULSES {
        assert!(ledger.advance());
    }
    assert!(!ledger.advance());
    let record = ledger.record();
    let failed: Vec<String> = record
        .checks
        .iter()
        .filter(|c| !c.pass)
        .map(|c| format!("{}: {}", c.name, c.detail))
        .collect();
    assert!(failed.is_empty(), "{failed:?}");
    assert_eq!(record.pulses, PULSES as usize);
}
