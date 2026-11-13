use crate::kernel::{NodeBuilder, Role, Surface, container};
use crate::layout::LayoutStyle;
use crate::widgets::Layout;

/// A rectangle that the embed host draws.
pub fn surface(key: u64) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .with(Surface { key })
        .layout(LayoutStyle::block().fill())
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
