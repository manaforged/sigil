#![allow(unused_crate_dependencies, clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::Host;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;

#[test]
fn routed_click_reaches_button_activate() {
    let clicked = Rc::new(Cell::new(false));
    let seen = clicked.clone();
    let mut ui = Harness::new(move || {
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 100.0))
            .child(button("hit me").activate({
                let seen = seen.clone();
                move || seen.set(true)
            }))
    })
    .pixels(200, 100);
    let target = ui.get_by_role(Role::Button, "hit me");
    ui.click(&target);

    assert!(
        clicked.get(),
        "routed click must reach the button's activate handler"
    );
}

#[test]
fn painted_and_surfaces_use_prelude_types() {
    Runtime::scope("embed_painted", || {
        let mut host = Host::new();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fixed_size(80.0, 24.0))
                .child(text("hello"))
                .child(surface(7))
        });
        host.layout((80.0, 24.0)).expect("layout");
        let scene = host.scene((80.0, 24.0)).expect("layout");
        assert!(
            scene.find_text("hello").is_some(),
            "expected hello in {scene:?}"
        );
        let holes = host.surfaces();
        let hole = holes.first().copied().expect("one surface hole");
        assert_eq!(hole.0, 7);
        assert!(hole.1.width > 0.0);
    });
}
