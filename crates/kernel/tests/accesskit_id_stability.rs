#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use ahash as _;
use criterion as _;
use hal as _;
use reactive as _;
use slotmap as _;
use smallvec as _;

use kernel::{Cx, Role, text};

#[test]
fn accesskit_id_is_stable_until_node_removal() {
    let mut cx = Cx::new_root();
    let id = cx.add(text("stable"));
    let accesskit_id = id.to_accesskit();

    for i in 0..100 {
        let node = cx.get_mut(id).expect("node remains live");
        node.a11y.focusable = i % 2 == 0;
        node.a11y.role = if i % 3 == 0 {
            Role::Label
        } else {
            Role::GenericContainer
        };
        assert_eq!(id.to_accesskit(), accesskit_id);
    }

    assert!(cx.remove(id));
    assert!(cx.get(id).is_none(), "stale slotmap generation is rejected");

    let replacement = cx.add(text("replacement"));
    assert_ne!(
        replacement.to_accesskit(),
        accesskit_id,
        "slot reuse must bump the generation encoded in AccessKit IDs"
    );
}
