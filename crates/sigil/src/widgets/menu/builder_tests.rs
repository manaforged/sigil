#![allow(clippy::unwrap_used)]

use super::*;
use crate::reactive::{Runtime, signal};

#[test]
fn anchor_is_stored_only_when_set() {
    Runtime::scope("menu-anchor-stored", || {
        let m = menu(signal(false)).item(MenuItem::new("Copy"));
        assert!(m.anchor.is_none());
        assert_eq!(m.anchor(120.0, 80.0).anchor, Some((120.0, 80.0)));
    });
}
