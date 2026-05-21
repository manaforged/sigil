//! Text-style resolution for layout measurement.

#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use kernel::Node;
use style::{StyleBinding, StyleEngine, TextStyle, Theme};

pub(crate) fn apply_text_binding(
    engine: StyleEngine<'_>,
    binding: &StyleBinding,
    style: &mut TextStyle,
) {
    // Sync point: typography atoms handled here must stay layout-affecting in
    // `cache_key.rs::is_layout_affecting_atom`.
    engine.apply_text(binding.static_slice(), style);
    engine.apply_text(binding.dynamic_slice(), style);
    engine.apply_text(binding.reactive_slice(), style);
}

pub(crate) fn theme_for(node: &Node) -> &Theme {
    node.get_extension::<Theme>().unwrap_or_else(default_theme)
}

fn default_theme<'theme>() -> &'theme Theme {
    static DEFAULT_THEME: OnceLock<Theme> = OnceLock::new();
    DEFAULT_THEME.get_or_init(Theme::default)
}
