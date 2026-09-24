#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::{Cx, text};

#[derive(Debug, PartialEq, Eq)]
struct Behavior(u8);

#[test]
fn extension_attach_get_overwrite_and_remove_roundtrip() {
    let mut cx = Cx::new_root();
    let id = cx.add(text("Reactive label"));
    let node = cx.get_mut(id).expect("node exists");

    assert_eq!(node.attach(Behavior(1)), None);
    assert!(node.has_extensions());
    assert_eq!(node.get_extension::<Behavior>(), Some(&Behavior(1)));

    assert_eq!(node.attach(Behavior(2)), Some(Behavior(1)));
    assert_eq!(node.get_extension::<Behavior>(), Some(&Behavior(2)));

    assert_eq!(node.remove_extension::<Behavior>(), Some(Behavior(2)));
    assert_eq!(node.get_extension::<Behavior>(), None);
}

#[test]
fn get_mut_does_not_advance_mutation_generation() {
    let mut cx = Cx::new_root();
    let id = cx.add(text("Reactive label"));
    let generation = cx.mutation_generation();

    let _node = cx.get_mut(id).expect("node exists");

    assert_eq!(cx.mutation_generation(), generation);
}

#[test]
fn explicit_mark_mutated_advances_mutation_generation() {
    let mut cx = Cx::new_root();
    let _id = cx.add(text("Reactive label"));
    let generation = cx.mutation_generation();

    cx.mark_mutated();

    assert_ne!(cx.mutation_generation(), generation);
}
