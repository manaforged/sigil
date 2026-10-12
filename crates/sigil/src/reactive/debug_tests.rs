use super::*;
use crate::reactive::runtime::OwnerSlot;

fn render(name: &'static str, depth: usize) -> String {
    let slot = OwnerSlot {
        parent: None,
        index_in_parent: None,
        children: Vec::new(),
        signals: Vec::new(),
        effects: Vec::new(),
        cleanups: Vec::new(),
        name,
        location: core::panic::Location::caller(),
    };
    render_line(&slot, depth)
}

#[test]
fn render_line_zero_counts_has_no_bracket() {
    let line = render("Root", 0);
    assert!(line.starts_with("Root  at "));
    assert!(!line.contains('['));
    assert!(line.ends_with('\n'));
}

#[test]
fn render_line_indents_two_spaces_per_depth() {
    assert!(render("Child", 0).starts_with("Child"));
    assert!(render("Child", 2).starts_with("    Child"));
}
