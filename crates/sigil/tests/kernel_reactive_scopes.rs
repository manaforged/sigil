#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::kernel::{Cx, container, text};
use sigil::reactive::{Runtime, signal};

#[test]
fn a_rebuild_queued_under_a_replaced_scope_never_mounts() {
    let mounts = Rc::new(Cell::new(0_u32));
    Runtime::scope("sigil_kernel_children_reactive_superseded", || {
        let outer = signal(0_u32);
        let inner = signal(0_u32);
        let counted = Rc::clone(&mounts);
        let mut cx = Cx::new_root();
        let _root = cx.add(container().children_reactive(move || {
            let _ = outer.get();
            let counted = Rc::clone(&counted);
            [container().children_reactive(move || {
                let value = inner.get();
                let counted = Rc::clone(&counted);
                [move || {
                    counted.set(counted.get() + 1);
                    text(value.to_string())
                }]
            })]
        }));
        cx.flush_reactive_children();
        let before = mounts.get();
        outer.set(1);
        inner.set(1);
        cx.flush_reactive_children();
        assert_eq!(
            mounts.get() - before,
            1,
            "only the rebuild from the live scope may mount"
        );
    });
}
