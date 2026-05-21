//! Extension trait that adds `.layout()` to `sigil-kernel::NodeBuilder`.
//!
//! Import this trait (or use `layout::prelude::*`) to attach a
//! [`LayoutStyle`] directly on the builder instead of post-hoc via
//! `node.attach(...)`:
//!
//! ```ignore
//! use layout::prelude::*;
//!
//! container()
//!     .layout(LayoutStyle::flex_row().fixed_size(320.0, 80.0))
//!     .child(text("hello"))
//! ```

use kernel::{NodeBuilder, TextBuilder};

use crate::LayoutStyle;

/// Adds `.layout()` to [`NodeBuilder`] so layout style is specifiable at
/// build time rather than as a separate post-hoc `node.attach(...)` call.
pub trait LayoutBuilderExt {
    /// Attach a [`LayoutStyle`] to this node at build time.
    #[must_use]
    fn layout(self, style: LayoutStyle) -> Self;
}

impl LayoutBuilderExt for NodeBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.with(style)
    }
}

impl LayoutBuilderExt for TextBuilder {
    fn layout(self, style: LayoutStyle) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(style);
        })
    }
}
