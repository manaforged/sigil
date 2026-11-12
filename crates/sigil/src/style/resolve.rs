use crate::kernel::{Node, NodeType};
use crate::style::{Color, RenderStyle, StyleBinding, StyleEngine, TextStyle, Theme, ToneContext};

kittest_pub! {
/// Resolves the paint style of a node from its style binding and theme.
#[must_use]
#[doc(hidden)]
pub fn render_style_for(
    node: &Node,
    inherited: &Theme,
    inherited_text: Option<Color>,
) -> RenderStyle {
    let mut render = RenderStyle::default();
    let theme = node.extension::<Theme>().unwrap_or(inherited);
    if matches!(node.kind, NodeType::Text { .. }) {
        render.text_color = Some(inherited_text.unwrap_or(theme.on_surface));
    }
    let Some(binding) = node.extension::<StyleBinding>() else {
        return render;
    };
    let engine = engine_for(node, theme);

    for atom in binding.cascade() {
        engine.apply(&[atom], &mut render);
    }

    if let Some(overlay) = binding.animator_overlay() {
        StyleEngine::overlay_render(overlay, &mut render);
    }
    render
}
}

/// Resolves the text style of a node on top of a base text style.
#[must_use]
pub(crate) fn text_style_for(node: &Node, inherited: &Theme, base: &TextStyle) -> TextStyle {
    let mut style = base.clone();
    let Some(binding) = node.extension::<StyleBinding>() else {
        return style;
    };

    let theme = node.extension::<Theme>().unwrap_or(inherited);
    let engine = engine_for(node, theme);

    for atom in binding.cascade() {
        engine.apply_text(&[atom], &mut style);
    }
    if let Some(overlay) = binding.animator_overlay() {
        StyleEngine::overlay_text(overlay, &mut style);
    }
    style
}

fn engine_for<'a>(node: &Node, theme: &'a Theme) -> StyleEngine<'a> {
    let tone = node
        .extension::<ToneContext>()
        .copied()
        .unwrap_or_default()
        .tone;
    StyleEngine::with_tone(theme, tone)
}
