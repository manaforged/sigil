//! Headless sigil-kernel tree example.

// Example prints its tree to stdout by design.
#![allow(clippy::print_stdout)]

use sigil_kernel::prelude::*;

fn main() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .role(Role::Main)
            .label("Hello tree")
            .child(text("Hello, world!"))
            .child(container().child(text("This is a node tree."))),
    );

    print!("{}", cx.dump_tree(root));
}
