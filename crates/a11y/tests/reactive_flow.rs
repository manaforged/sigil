#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Production a11y snapshot: `tree()` peeks live signals.
//!
//! Launch remaps the AccessKit tree on paint. There is no second
//! observer/publisher in the reactive runtime.

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns
)]

use accesskit::Role;
use sigil_a11y::{snapshot, tree};
use sigil_kernel::{Cx, container};
use sigil_reactive::{Runtime, signal};

#[test]
fn checkbox_signal_change_shows_in_tree() {
    Runtime::scope("checkbox_signal_change_shows_in_tree", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let _ = cx.add(
            container()
                .role(Role::CheckBox)
                .label("Accept terms")
                .checked(checked),
        );

        let first = snapshot(&tree(&cx, None, &|_| None));
        assert!(
            first.contains("Toggled=False"),
            "initial tree shows Toggled=False, got:\n{first}"
        );

        checked.set(true);
        let second = snapshot(&tree(&cx, None, &|_| None));
        assert!(
            second.contains("Toggled=True"),
            "peeked tree shows Toggled=True, got:\n{second}"
        );
        assert!(second.contains("Role=CheckBox"), "got:\n{second}");
        assert!(second.contains("Name=\"Accept terms\""), "got:\n{second}");
    });
}
