#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;
use std::time::{Duration, Instant};

use sigil::prelude::*;
use sigil::widgets::highlight;
use sigil_kittest::Harness;

const STACK: usize = 8 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(60);
const BUDGET: Duration = if cfg!(debug_assertions) {
    Duration::from_secs(5)
} else {
    Duration::from_millis(500)
};

fn mount(name: &str, source: String) -> (Duration, String) {
    let (done, finished) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name(name.to_string())
        .stack_size(STACK)
        .spawn(move || {
            let started = Instant::now();
            let mut ui = Harness::new(move || markdown(signal(source))).pixels(800, 600);
            ui.settle();
            done.send((started.elapsed(), ui.text())).expect("receiver");
        })
        .expect("spawn");
    let shown = finished
        .recv_timeout(DEADLINE)
        .unwrap_or_else(|_| panic!("{name} did not mount within {DEADLINE:?}"));
    worker.join().expect("mount thread panicked");
    shown
}

fn json_line(bytes: usize) -> String {
    let mut line = String::from("[");
    let mut index = 0;
    while line.len() < bytes {
        line.push_str(&format!("{{\"id\":{index},\"name\":\"county {index}\"}},"));
        index += 1;
    }
    line.push_str("{}]");
    line
}

fn fenced(body: &str) -> String {
    format!("Before\n\n```json\n{body}\n```\n\nAfter\n")
}

fn assert_bounded(name: &str, source: String) -> String {
    let (took, shown) = mount(name, source);
    println!("{name}: mount and settle took {took:?}");
    assert!(
        shown.contains("Before"),
        "{name}: text above the block is missing"
    );
    assert!(
        shown.contains("county 0"),
        "{name}: the block's first line is missing"
    );
    assert!(
        shown.contains("After"),
        "{name}: text below the block is missing"
    );
    assert!(
        took <= BUDGET,
        "{name}: mount and settle took {took:?}, over {BUDGET:?}"
    );
    shown
}

#[test]
fn a_700_kb_single_line_fence_mounts_within_budget() {
    let shown = assert_bounded("one-line", fenced(&json_line(700 * 1024)));
    assert!(shown.contains('…'), "the cut line has no marker");
    assert!(
        !shown.contains("county 9999"),
        "the whole line was laid out"
    );
}

#[test]
fn a_700_kb_fence_of_20k_lines_mounts_within_budget() {
    let body: Vec<String> = (0..20_000)
        .map(|index| format!("  {{\"id\": {index}, \"name\": \"county {index}\"}},"))
        .collect();
    let shown = assert_bounded("many-lines", fenced(&body.join("\n")));
    assert!(
        shown.contains("county 499"),
        "the first 500 lines are not all shown"
    );
    assert!(!shown.contains("county 19999"), "every line was laid out");
    assert!(
        shown.contains("19500 more lines not shown"),
        "no count of the hidden lines"
    );
}

#[test]
fn highlighting_a_700_kb_line_finishes_within_budget() {
    let line = json_line(700 * 1024);
    let width = line.len();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let started = Instant::now();
        let marks = highlight("json", &line);
        done.send((started.elapsed(), marks)).expect("receiver");
    });
    let (took, marks) = finished
        .recv_timeout(DEADLINE)
        .unwrap_or_else(|_| panic!("highlight did not return within {DEADLINE:?}"));
    println!("highlight: 700 KB line took {took:?}");
    let last = marks[0].last().expect("marks");
    assert!(
        last.end <= width && width - last.end < 40,
        "last mark ends at column {} of {width}",
        last.end
    );
    assert!(took <= BUDGET, "highlight took {took:?}, over {BUDGET:?}");
}

#[test]
fn a_200_kb_paragraph_of_inline_code_mounts_within_budget() {
    let words: Vec<String> = (0..14_000)
        .map(|index| format!("`county {index}`"))
        .collect();
    let source = format!("Before\n\n{}\n\nAfter\n", words.join(" "));
    assert_bounded("inline-code", source);
}
