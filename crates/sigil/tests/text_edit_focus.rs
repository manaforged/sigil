#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil_kittest::Harness;

fn focused_role(ui: &mut Harness) -> Option<Role> {
    ui.focused().map(|node| node.role())
}

#[test]
fn autofocus_focuses_the_editor_when_it_mounts() {
    let mut ui = Harness::new(|| text_edit(signal(String::new())).autofocus());
    assert_eq!(focused_role(&mut ui), Some(Role::TextInput));
}

#[test]
fn focus_when_moves_focus_to_the_editor() {
    let mut ui = Harness::new(|| {
        let trigger = signal(0_u64);
        column()
            .child(button("Edit").activate(move || trigger.update(|n| *n += 1)))
            .child(text_edit(signal(String::new())).focus_when(trigger))
    });
    assert_ne!(focused_role(&mut ui), Some(Role::TextInput));
    ui.click("Edit");
    assert_eq!(focused_role(&mut ui), Some(Role::TextInput));
}
