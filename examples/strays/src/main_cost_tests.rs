use std::time::Instant;

use crate::scan::Scanner;

/// A scan runs on the frame clock, so it has to cost less than a frame.
#[test]
fn a_repeat_scan_stays_under_a_frame() {
    let mut reader = Scanner::new();
    let start = Instant::now();
    let rows = reader.read();
    let first = start.elapsed();

    let repeat_start = Instant::now();
    for _ in 0..5 {
        drop(reader.read());
    }
    let repeat = repeat_start.elapsed() / 5;
    println!(
        "scan: first={first:?} repeat={repeat:?} rows={}",
        rows.len()
    );
    assert!(
        repeat.as_millis() < 33,
        "a repeat scan must fit in a frame, took {repeat:?}"
    );
}
