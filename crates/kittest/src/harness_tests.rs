use std::time::Duration;

use sigil::prelude::*;

use crate::{Harness, Query};

fn counter() -> impl IntoNode {
    let count = signal(0_i32);
    let label = derived(move || format!("Count: {}", count.get()));
    container()
        .child(text(label))
        .child(button("Increment").activate(move || count.set(count.peek() + 1)))
}

fn form() -> impl IntoNode {
    let name = signal(String::new());
    let agree = signal(false);
    container()
        .layout(LayoutStyle::flex_column())
        .child(field("Name", text_field(name)))
        .child(checkbox(agree).label("Agree"))
        .child(button("Save"))
}

#[test]
fn click_by_role_updates_the_terminal_grid() {
    let mut ui = Harness::new(counter).size(40, 6);
    let increment = ui.get_by_role(Role::Button, "Increment");
    ui.click(&increment).click(&increment);
    assert!(ui.text().contains("Count: 2"), "{}", ui.text());
    assert_eq!(ui.find_all(&Query::text("Count: 2")).len(), 1);
}

#[test]
fn pixel_host_routes_the_same_click() {
    let mut ui = Harness::new(counter).pixels(320, 120);
    let increment = ui.get_by_role(Role::Button, "Increment");
    ui.click(&increment);
    assert_eq!(ui.get_by_text("Count: 1").value(), Some("Count: 1"));
}

#[test]
fn typing_fills_the_labelled_field() {
    let mut ui = Harness::new(form).size(40, 12);
    let field = ui.get_by_role(Role::TextInput, "Name");
    ui.click(&field).type_text("Ada");
    let typed = ui.get_by_role(Role::TextInput, "Name");
    assert_eq!(typed.value(), Some("Ada"));
    assert!(typed.is_focused());
}

#[test]
fn click_toggles_a_checkbox() {
    let mut ui = Harness::new(form).size(40, 12);
    let agree = ui.get_by_role(Role::CheckBox, "Agree");
    assert_eq!(agree.is_checked(), Some(false));
    ui.click(&agree);
    assert_eq!(ui.refresh(&agree).and_then(|n| n.is_checked()), Some(true));
}

#[test]
fn focus_next_walks_the_tab_order() {
    let mut ui = Harness::new(form).size(40, 12);
    let first = ui.focused().map(|node| node.role());
    ui.focus_next();
    let second = ui.focused().map(|node| node.role());
    assert_ne!(first, second);
    ui.focus_previous();
    assert_eq!(ui.focused().map(|node| node.role()), first);
}

#[test]
fn advance_fires_intervals_on_the_virtual_clock() {
    let ticks = || {
        let n = signal(0_u32);
        interval(Duration::from_secs(1), move || n.set(n.peek() + 1));
        text(derived(move || format!("ticks {}", n.get())))
    };
    let mut ui = Harness::new(ticks).size(20, 2);
    assert!(ui.text().contains("ticks 0"), "{}", ui.text());
    ui.advance(Duration::from_secs(3));
    assert!(ui.text().contains("ticks 3"), "{}", ui.text());
}

#[test]
fn snapshot_compares_the_grid_to_a_text_golden() {
    let mut ui = Harness::new(counter).size(24, 4);
    ui.snapshot("harness_counter");
}

#[test]
#[should_panic(expected = "expected one node for role Button name \"Missing\"")]
fn get_fails_with_the_tree_when_nothing_matches() {
    let mut ui = Harness::new(counter).size(20, 4);
    let _ = ui.get_by_role(Role::Button, "Missing");
}

#[test]
fn pixel_harness_renders_at_its_size() {
    let mut ui = Harness::new(counter).pixels(160, 60);
    let Ok(Some(image)) = ui.image() else {
        return;
    };
    assert_eq!(image.dimensions(), (160, 60));
}

#[test]
fn a_label_finds_the_control_it_names() {
    let mut ui = Harness::new(form).size(40, 12);
    ui.type_into("Name", "Ada").click("Agree");
    assert_eq!(ui.get_by_role(Role::TextInput, "Name").value(), Some("Ada"));
    assert_eq!(
        ui.get_by_role(Role::CheckBox, "Agree").is_checked(),
        Some(true)
    );
}

#[test]
#[should_panic(expected = "names 2 controls")]
fn a_label_on_two_controls_fails() {
    let mut ui = Harness::new(|| container().child(button("Delete")).child(button("Delete")));
    ui.click("Delete");
}

#[test]
fn expect_text_waits_for_a_task() {
    let mut ui = Harness::new(|| {
        let load = task(|| async {
            sigil::tasks::sleep(Duration::from_millis(200)).await;
            Ok::<_, String>("done".to_owned())
        });
        container()
            .child(button("Load").activate(move || load.run()))
            .child(text(move || match load.status() {
                Status::Ok(value) => value,
                _ => String::new(),
            }))
    });
    ui.click("Load");
    assert!(ui.query(&Query::text("done")).is_none());
    ui.expect_text("done");
}
