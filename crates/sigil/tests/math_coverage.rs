#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use accesskit::Role as AkRole;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

const COMMON: &str = include_str!("data/math_common.tsv");
const REQUIRED: usize = 135;

fn typeset(source: &str) -> bool {
    let source = source.to_string();
    Owner::scope("math-coverage", || {
        let mut ui =
            Harness::new(move || column().child(math(signal(source.clone())))).pixels(1600, 600);
        ui.settle();
        ui.accessibility_tree().nodes.iter().any(|(_, node)| {
            node.role() == AkRole::Figure
                && node.label() == Some("Math")
                && node.bounds().is_some_and(|area| area.width() > 1.0)
        })
    })
}

#[test]
fn the_common_latex_constructs_draw_as_formulas() {
    let rows: Vec<(&str, &str)> = COMMON
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    assert_eq!(rows.len(), 138, "corpus rows");
    let fallbacks: Vec<&str> = rows
        .iter()
        .filter(|(_, source)| !typeset(source))
        .map(|(construct, _)| *construct)
        .collect();
    let drawn = rows.len() - fallbacks.len();
    println!("math coverage: {drawn}/{} typeset", rows.len());
    assert!(
        drawn >= REQUIRED,
        "math coverage: {drawn}/{} typeset, need {REQUIRED}; fallback: {fallbacks:?}",
        rows.len()
    );
}
