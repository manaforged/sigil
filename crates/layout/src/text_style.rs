//! Text-style resolution for layout measurement.

#![allow(clippy::redundant_pub_crate)]

use std::sync::OnceLock;

use sigil_kernel::{Node, TextContent};
use sigil_style::{StyleBinding, StyleEngine, TextStyle, Theme};
use taffy::prelude::{AvailableSpace, Size as TaffySize};

use crate::TextMeasure;

#[derive(Clone, Debug)]
pub(crate) struct LayoutContext {
    pub(crate) text_content: Option<TextContent>,
    pub(crate) text_style: Option<TextStyle>,
}

pub(crate) fn measure_text(
    measurer: &dyn TextMeasure,
    known: TaffySize<Option<f32>>,
    space: TaffySize<AvailableSpace>,
    context: Option<&mut LayoutContext>,
) -> TaffySize<f32> {
    let text_size = context
        .and_then(|context| {
            let source = context.text_content.as_ref()?;
            Some(context.text_style.as_ref().map_or_else(
                || measurer.measure(source, space),
                |style| measurer.measure_styled(source, style, space),
            ))
        })
        .unwrap_or_default();

    TaffySize {
        width: known.width.unwrap_or(text_size.width),
        height: known.height.unwrap_or(text_size.height),
    }
}

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
