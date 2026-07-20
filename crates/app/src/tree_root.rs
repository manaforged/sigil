//! `TreeRoot` — sealed trait accepting the tree-function shapes that
//! `launch` will take.
//!
//! Two shapes cover everything user code actually writes:
//!
//! ```text
//! sigil::launch(cfg, |cx| app_tree(cx))        // FnOnce(&mut Cx) -> NodeId
//! sigil::launch(cfg, container().child(text("hi")))  // NodeBuilder
//! ```

use sigil_kernel::{Cx, NodeBuilder, NodeId};

mod sealed {
    pub trait Sealed {}
    impl<F: FnOnce(&mut sigil_kernel::Cx) -> sigil_kernel::NodeId> Sealed for F {}
    impl Sealed for sigil_kernel::NodeBuilder {}
}

/// Anything that can install itself under a `Cx` and return the root
/// node. Sealed — only the two built-in impls exist.
pub trait TreeRoot: sealed::Sealed {
    /// Install this tree under the given context and return the root ID.
    fn install(self, cx: &mut Cx) -> NodeId;
}

impl<F> TreeRoot for F
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn install(self, cx: &mut Cx) -> NodeId {
        self(cx)
    }
}

impl TreeRoot for NodeBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}
