mod builder;
mod spawn;
mod state;
mod style;

pub use builder::{FctNode, FloatingTextBuilder, floating_text};
pub use spawn::{FctSpawn, Particle};
pub use state::{FCT_POOL_CAPACITY, FctState};
pub use style::FloaterStyle;

#[cfg(test)]
#[path = "floating_text_tests.rs"]
mod tests;
