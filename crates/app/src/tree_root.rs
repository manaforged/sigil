//! `TreeRoot` — sealed trait for the tree `launch`, `run_tui`, and
//! [`Host::mount`](crate::Host::mount) accept.
//!
//! Closures keep argument inference. `text`, `image`, and `container`
//! install directly. Other widgets use [`root`]:
//!
//! ```text
//! sigil::launch(cfg, |cx| app_tree(cx))
//! sigil::launch(cfg, text("hi"))
//! sigil::launch(cfg, root(button("Go")))
//! ```

use sigil_kernel::{Cx, ImageBuilder, IntoNode, NodeBuilder, NodeId, TextBuilder};

mod sealed {
    pub trait Sealed {}
    impl<F: FnOnce(&mut sigil_kernel::Cx) -> sigil_kernel::NodeId> Sealed for F {}
    impl Sealed for sigil_kernel::NodeBuilder {}
    impl Sealed for sigil_kernel::TextBuilder {}
    impl Sealed for sigil_kernel::ImageBuilder {}
    impl<N: sigil_kernel::IntoNode> Sealed for super::Root<N> {}
}

/// Lift any [`IntoNode`] into a [`TreeRoot`].
///
/// Closures stay unwrapped so `cx` infers. Use this for `button`,
/// `splitter`, and other widgets:
/// `sigil::launch(cfg, root(button("Go")))`.
#[must_use]
pub fn root(node: impl IntoNode) -> impl TreeRoot {
    Root(node)
}

struct Root<N: IntoNode>(N);

impl<N: IntoNode> TreeRoot for Root<N> {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self.0)
    }
}

/// Anything that can install itself under a `Cx` and return the root
/// node. Sealed — a closure, [`TextBuilder`], [`ImageBuilder`],
/// [`NodeBuilder`], or [`root`].
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

impl TreeRoot for TextBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}

impl TreeRoot for ImageBuilder {
    fn install(self, cx: &mut Cx) -> NodeId {
        cx.add(self)
    }
}
