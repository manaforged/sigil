#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use accesskit::Role as AkRole;
use sigil::events::{Key, LogicalKey};
use sigil::prelude::*;
use sigil::widgets::{ListRowSlots, list_row_with, virtual_list_bind};
use sigil_kittest::Harness;

#[test]
fn a_required_field_is_exposed_as_required() {
    let mut ui = Harness::new(|| {
        column()
            .child(field("Given name", text_field(signal(String::new()))).required(true))
            .child(field("Nickname", text_field(signal(String::new()))))
    })
    .size(40, 8);
    let inputs: Vec<bool> = ui
        .accessibility_tree()
        .nodes
        .into_iter()
        .filter(|(_, node)| node.role() == AkRole::TextInput)
        .map(|(_, node)| node.is_required())
        .collect();
    assert_eq!(inputs, vec![true, false]);
}

#[test]
fn an_activatable_list_row_takes_focus_and_runs_on_enter() {
    let opened = Rc::new(Cell::new(0_u32));
    let count = Rc::clone(&opened);
    let mut ui = Harness::new(move || {
        let count = Rc::clone(&count);
        list_row_with(
            false,
            "# design",
            ListRowSlots {
                activate: Some(Rc::new(move || count.set(count.get() + 1))),
                ..ListRowSlots::default()
            },
        )
    })
    .size(40, 4);
    ui.focus_next();
    assert!(ui.focused().is_some(), "Tab did not reach the row");
    ui.press(Key::Enter);
    ui.press(LogicalKey::character(" "));
    assert_eq!(opened.get(), 2);
}

#[test]
fn virtual_list_rows_report_their_position_in_the_set() {
    let mut ui = Harness::new(|| {
        let items = signal((0..50).map(|at| format!("item {at}")).collect::<Vec<_>>());
        virtual_list_bind(items, |_, item: &String| text(item.clone())).row_height(16.0)
    })
    .size(40, 10);
    let mut positions: Vec<(usize, usize)> = ui
        .accessibility_tree()
        .nodes
        .into_iter()
        .filter_map(|(_, node)| Some((node.position_in_set()?, node.size_of_set()?)))
        .collect();
    positions.sort_unstable();
    assert_eq!(positions.first(), Some(&(1, 50)));
    assert!(positions.iter().all(|(_, size)| *size == 50));
}
