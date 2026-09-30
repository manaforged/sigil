#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;

fn save_form(saves: Signal<u32>) -> impl IntoNode {
    let name = signal(String::new());
    column().child(text_field(name).placeholder("Name")).child(
        button("Save")
            .disabled(move || name.get().trim().is_empty())
            .activate(move || saves.update(|n| *n += 1)),
    )
}

#[test]
fn a_closure_drives_disabled_until_the_field_has_text() {
    Runtime::scope("reactive-props-disabled", || {
        let saves = signal(0_u32);
        let mut ui = Harness::new(move || save_form(saves));
        let save = ui.get_by_role(Role::Button, "Save");
        assert!(save.is_disabled(), "Save must start disabled");
        ui.click(&save).settle();
        assert_eq!(saves.get(), 0, "a disabled Save must not fire");

        let field = ui.get_by_role(Role::TextInput, "Name");
        ui.click(&field).type_text("Ada").settle();
        let enabled = ui.get_by_role(Role::Button, "Save");
        assert!(
            !enabled.is_disabled(),
            "Save must enable once the field has text"
        );
        ui.click(&enabled).settle();
        assert_eq!(saves.get(), 1, "an enabled Save must fire once");
    });
}

#[test]
fn a_plain_bool_disables_a_widget() {
    let mut ui = Harness::new(|| button("Save").disabled(true));
    assert!(ui.get_by_role(Role::Button, "Save").is_disabled());
}
