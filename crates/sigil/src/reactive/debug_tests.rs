use super::*;
use crate::reactive::runtime::OwnerSlot;

#[test]
fn render_line_zero_counts_has_no_bracket() {
    let loc = core::panic::Location::caller();
    let slot = OwnerSlot {
        parent: None,
        index_in_parent: None,
        children: Vec::new(),
        signals: Vec::new(),
        effects: Vec::new(),
        entities: Vec::new(),
        observers: Vec::new(),
        cleanups: Vec::new(),
        name: "Root",
        location: loc,
    };
    let line = render_line(&slot, 0);
    assert!(line.starts_with("Root  at "));
    assert!(!line.contains('['));
    assert!(line.ends_with('\n'));
}

#[test]
fn render_line_indents_two_spaces_per_depth() {
    let loc = core::panic::Location::caller();
    let slot = OwnerSlot {
        parent: None,
        index_in_parent: None,
        children: Vec::new(),
        signals: Vec::new(),
        effects: Vec::new(),
        entities: Vec::new(),
        observers: Vec::new(),
        cleanups: Vec::new(),
        name: "Child",
        location: loc,
    };
    let d0 = render_line(&slot, 0);
    let d2 = render_line(&slot, 2);
    assert!(d0.starts_with("Child"));
    assert!(d2.starts_with("    Child"));
}
