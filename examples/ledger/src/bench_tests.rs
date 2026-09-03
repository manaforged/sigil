#![allow(clippy::unwrap_used)]

use bench_spec::ledger::PULSES;
use sigil::testing::with_cx;

use crate::App;
use crate::bench::Driver;
use crate::view::tree;

#[test]
fn the_script_replays_to_the_expected_state() {
    with_cx("ledger-bench-replay", |cx| {
        let app = App::new(2_000);
        let _root = cx.add(tree(&app));
        let out = std::env::temp_dir().join(format!("ledger-bench-{}.json", std::process::id()));
        let mut driver = Driver::new(app, out.clone());
        let mut done = false;
        for _ in 0..PULSES {
            assert!(!done, "finished before the last pulse");
            done = driver.pulse();
        }
        assert!(done);
        assert!(driver.pulse(), "stays finished");
        let record = driver.record();
        let failed: Vec<String> = record
            .checks
            .iter()
            .filter(|c| !c.pass)
            .map(|c| format!("{}: {}", c.name, c.detail))
            .collect();
        assert!(failed.is_empty(), "{failed:?}");
        assert_eq!(record.pulses, PULSES as usize);
        assert!(out.exists());
        std::fs::remove_file(out).unwrap();
    });
}
