#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use ahash as _;
use criterion as _;
use sigil_reactive as _;
use slotmap as _;
use smallvec as _;

use sigil_core::{Cx, text};

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
