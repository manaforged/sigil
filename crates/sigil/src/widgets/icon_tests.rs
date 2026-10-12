#![allow(clippy::unwrap_used)]

use super::*;
use crate::kernel::Cx;
use crate::reactive::Runtime;

#[test]
fn decorative_icon_is_one_generic_container() {
    Runtime::scope("icon-decorative-role", || {
        let mut cx = Cx::new_root();
        let id = icon("\u{2699}").build(&mut cx);
        let node = cx.get(id).expect("icon node exists");
        assert_eq!(node.a11y.role, crate::kernel::Role::GenericContainer);
        assert_eq!(node.children().len(), 1, "icon wraps one glyph text node");
    });
}

#[test]
fn labeled_icon_promotes_to_image_role() {
    Runtime::scope("icon-labeled-role", || {
        let mut cx = Cx::new_root();
        let id = icon("\u{2699}").label("Settings").build(&mut cx);
        let node = cx.get(id).expect("icon node exists");
        assert_eq!(node.a11y.role, crate::kernel::Role::Image);
    });
}
