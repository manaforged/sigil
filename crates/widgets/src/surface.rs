//! Layout hole for a native view.

use sigil_kernel::{NodeBuilder, Role, Surface, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};

/// Empty box the GPU and TUI do not paint. After layout,
/// `Host::surfaces` returns `(key, rect)`. The caller places a native
/// view in that rect, or ignores it.
#[must_use]
pub fn surface(key: u64) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .with(Surface { key })
        .layout(LayoutStyle::block().fill())
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
