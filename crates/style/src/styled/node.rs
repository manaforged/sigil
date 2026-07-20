//! `NodeStyledBuilder` — the wrapper that gives `NodeBuilder` a [`Style`]
//! accumulator and the [`Styleable::styled`] adapter that
//! constructs it.
//!
//! `NodeBuilder` does not own a `Style` field today; instead it queues
//! `mutate` closures applied at `build()` time. The G6 contract is "the
//! trait's `style_mut()` method returns `&mut Style`"; we satisfy that with
//! a small `NodeStyledBuilder` newtype that owns a `Style` accumulator
//! and forwards `IntoNode` to the underlying `NodeBuilder` with the
//! accumulator attached at build time via the existing `NodeStyleExt`.

use sigil_kernel::NodeBuilder;

use super::{StateHost, StyleAccess};
use crate::{ClassSource, NodeStyleExt, Style};

/// Wrapper that gives a [`NodeBuilder`] a [`Style`] accumulator so it
/// can implement [`crate::Styled`]. Returned by [`crate::NodeStyledBuilder::wrap`].
///
/// At `IntoNode::build` time the accumulator is consumed and attached
/// to the kernel node via the existing [`crate::NodeStyleExt::style`]
/// path — the install seam is unchanged from DR-049 §9.
pub struct NodeStyledBuilder {
    inner: NodeBuilder,
    style: Style,
}

impl NodeStyledBuilder {
    /// Wrap a kernel [`NodeBuilder`] for fluent styling.
    #[must_use]
    pub fn wrap(inner: NodeBuilder) -> Self {
        Self {
            inner,
            style: Style::new(),
        }
    }

    /// Borrow the accumulated style.
    #[must_use]
    pub const fn style(&self) -> &Style {
        &self.style
    }
    /// Consume the wrapper and return the underlying [`NodeBuilder`]
    /// with the accumulated style attached.
    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        attach_style(self.inner, self.style)
    }
}

impl StyleAccess for NodeStyledBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl StateHost for NodeStyledBuilder {}

/// Adapter trait — call [`Styleable::styled`] on a
/// [`NodeBuilder`] to start fluent styling against it.
pub trait Styleable: Sized {
    /// Begin fluent styling on this builder.
    #[must_use]
    fn styled(self) -> NodeStyledBuilder;
}

impl Styleable for NodeBuilder {
    fn styled(self) -> NodeStyledBuilder {
        NodeStyledBuilder::wrap(self)
    }
}

impl sigil_kernel::IntoNode for NodeStyledBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

/// Attach an accumulated [`Style`] to a [`NodeBuilder`] via the existing
/// `NodeStyleExt::style` install path. State-slot overlays are drained
/// into a Rest-pinned slot cell: the Rest slice applies unconditionally,
/// while interaction-state slices stay inert until the node is wired for
/// interaction (`interactive()`, 5/5 Wave 1 T2) — nothing is silently
/// dropped (the pre-T2 "G6.2 deferral" used to discard them).
fn attach_style(builder: NodeBuilder, style: Style) -> NodeBuilder {
    if style.is_empty() {
        return builder;
    }
    let mut style = style;
    let overlay = style.take_state_overlay();
    let builder = if overlay.is_empty() {
        builder
    } else {
        builder.state_overlay(overlay, rest_slot_cell())
    };
    // Base atoms go through the existing class-source path.
    let base_atoms = style.into_atoms();
    if base_atoms.is_empty() {
        return builder;
    }
    builder.class(ClassSource::Owned(Box::new(base_atoms)))
}

/// Slot cell pinned to Rest for non-interactive installs: the resolver
/// reads it and applies the Rest slice only.
fn rest_slot_cell() -> crate::ReactiveStateSlot {
    use std::cell::Cell;
    use std::rc::Rc;

    Rc::new(Cell::new(crate::StateSlot::Rest))
}
