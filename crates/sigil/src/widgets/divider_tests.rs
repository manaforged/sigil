#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::{Cx, IntoNode};
use crate::reactive::Owner;

#[test]
fn dividers_are_decorative_not_focusable_splitters() {
    Owner::scope("divider-build", || {
        let mut cx = Cx::new_root();
        for (id, kind) in [
            (
                divider(Orientation::Horizontal).build(&mut cx),
                "horizontal",
            ),
            (divider(Orientation::Vertical).build(&mut cx), "vertical"),
        ] {
            let node = cx.get(id).expect("divider node exists");
            assert!(
                !node.a11y.hittable,
                "{kind} divider must opt out of the a11y tree"
            );
            assert!(
                !node.a11y.focusable,
                "{kind} divider must not be keyboard-focusable"
            );
        }
    });
}
