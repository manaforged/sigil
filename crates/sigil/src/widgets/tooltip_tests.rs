#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::container as kernel_container;
use crate::layout::Display;
use crate::reactive::Owner;
use crate::widgets::popup::shown_when;

#[test]
fn tooltip_builds_with_target_child_on_either_side() {
    Owner::scope("tooltip-build", || {
        let _below = tooltip("Hint")
            .below()
            .child(kernel_container())
            .into_node_builder();
        let _above = tooltip("Save the file")
            .child(kernel_container())
            .into_node_builder();
    });
}

#[test]
fn tooltip_shortcut_appears_in_description() {
    let with = bubble_description("Save the file", Some("Ctrl+S"));
    assert_eq!(with, "Save the file (Ctrl+S)");
    let without = bubble_description("Save the file", None);
    assert_eq!(without, "Save the file");
    let empty = bubble_description("Save the file", Some(""));
    assert_eq!(empty, "Save the file");
}

#[test]
fn shown_when_gates_display_none() {
    assert!(shown_when(true).is_empty());
    assert_eq!(shown_when(false).len(), 1);
    assert_eq!(shown_when(false)[0], StyleAtom::Display(Display::None));
}
