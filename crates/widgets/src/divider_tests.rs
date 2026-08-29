#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use sigil_kernel::{Cx, IntoNode};
use sigil_reactive::Runtime;

#[test]
fn horizontal_is_decorative_not_a_focusable_splitter() {
    Runtime::scope("divider-horizontal-build", || {
        let mut cx = Cx::new_root();
        let id = horizontal().build(&mut cx);
        let node = cx.get(id).expect("divider node exists");
        assert!(
            !node.a11y.hittable,
            "horizontal divider must opt out of the a11y tree"
        );
        assert!(
            !node.a11y.focusable,
            "horizontal divider must not be keyboard-focusable"
        );
    });
}

#[test]
fn vertical_is_decorative_not_a_focusable_splitter() {
    Runtime::scope("divider-vertical-build", || {
        let mut cx = Cx::new_root();
        let id = vertical().build(&mut cx);
        let node = cx.get(id).expect("divider node exists");
        assert!(
            !node.a11y.hittable,
            "vertical divider must opt out of the a11y tree"
        );
        assert!(
            !node.a11y.focusable,
            "vertical divider must not be keyboard-focusable"
        );
    });
}
