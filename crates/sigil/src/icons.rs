#![allow(clippy::redundant_pub_crate)]

pub(crate) mod builtin;
mod icon;

pub use icon::Icon;

#[cfg(test)]
#[path = "icons/lib_tests.rs"]
mod tests;
