use std::cell::Cell;
use std::rc::Rc;

use sigil::events::Key;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::widgets::tag_input;
use sigil_kittest::{Harness, Query};

const OPTIONS: [&str; 4] = ["bug", "docs", "feature", "question"];

fn open() -> (Harness, Signal<Vec<String>>) {
    let slot = Rc::new(Cell::new(None::<Signal<Vec<String>>>));
    let export = Rc::clone(&slot);
    let mut ui = Harness::new(move || {
        let labels = signal(Vec::<String>::new());
        export.set(Some(labels));
        column().child(field(
            "Labels",
            tag_input(OPTIONS.map(String::from), labels).placeholder("Add a label"),
        ))
    })
    .size(48, 16);
    ui.settle();
    let labels = slot.get().expect("labels signal");
    (ui, labels)
}

fn type_in(ui: &mut Harness, text: &str) {
    let input = ui.get(&Query::role(Role::TextInput));
    ui.click(&input).type_text(text).settle();
}

fn tags(labels: Signal<Vec<String>>) -> Vec<String> {
    labels.peek()
}

#[test]
fn typing_filters_and_enter_or_click_adds_a_tag() {
    let (mut ui, labels) = open();
    type_in(&mut ui, "do");
    ui.press(Key::Enter).settle();
    assert_eq!(tags(labels), ["docs"]);
    ui.expect_text("docs");
    let input = ui.get(&Query::role(Role::TextInput));
    assert_eq!(input.value().unwrap_or_default(), "");
    type_in(&mut ui, "fea");
    let option = ui.get_by_role(Role::ListBoxOption, "feature");
    ui.click(&option).settle();
    assert_eq!(tags(labels), ["docs", "feature"]);
}

#[test]
fn each_tag_removes_with_its_button_and_backspace_removes_the_last() {
    let (mut ui, labels) = open();
    for query in ["bu", "qu", "fe"] {
        type_in(&mut ui, query);
        ui.press(Key::Enter).settle();
    }
    assert_eq!(tags(labels), ["bug", "question", "feature"]);
    let remove = ui.get_by_role(Role::Button, "Remove question");
    ui.click(&remove).settle();
    assert_eq!(tags(labels), ["bug", "feature"]);
    let input = ui.get(&Query::role(Role::TextInput));
    ui.click(&input).press(Key::Backspace).settle();
    assert_eq!(tags(labels), ["bug"]);
    assert!(
        ui.query(&Query::role(Role::Button).name("Remove feature"))
            .is_none(),
        "the removed tag still shows"
    );
}
