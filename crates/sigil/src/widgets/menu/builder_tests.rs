#![allow(clippy::unwrap_used)]

use super::*;
use crate::reactive::{Runtime, signal};

#[test]
fn anchor_stores_position_for_into_node_builder() {
    Runtime::scope("menu-anchor-stored", || {
        let m = menu(signal(false))
            .item(MenuItem::new("Copy"))
            .anchor(120.0, 80.0);
        assert_eq!(m.anchor, Some((120.0, 80.0)));
    });
}

#[test]
fn unanchored_menu_keeps_none() {
    Runtime::scope("menu-anchor-default-none", || {
        let m = menu(signal(false)).item(MenuItem::new("Copy"));
        assert!(m.anchor.is_none());
    });
}
