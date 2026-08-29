use sigil_kernel::{NodeBuilder, Role, Surface, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};

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
