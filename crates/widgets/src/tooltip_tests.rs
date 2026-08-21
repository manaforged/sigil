#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use sigil_kernel::container as kernel_container;
use sigil_reactive::Runtime;

#[test]
fn tooltip_builds_with_target_child() {
    Runtime::scope("tooltip-build", || {
        let _builder = tooltip("Save the file")
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
fn visibility_atoms_gate_display_none() {
    assert!(visibility_atoms(true).is_empty());
    assert_eq!(visibility_atoms(false).len(), 1);
    assert_eq!(
        visibility_atoms(false)[0],
        StyleAtom::Display(DisplayKind::None)
    );
}

#[test]
fn clamp_top_pins_bubble_at_viewport_edge() {
    Runtime::scope("tooltip-clamp-top", || {
        let _builder = tooltip("Hint")
            .clamp_top()
            .child(kernel_container())
            .into_node_builder();
    });
}
