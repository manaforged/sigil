use crate::kernel::{Node, NodeId, NodeType};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::style::{
    RenderStyle, Rgba8, StyleAtom, StyleBinding, StyleEngine, TextStyle, Theme, ToneContext,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum UnsupportedSlot {
    UnknownThemeRef(&'static str),
}

#[cfg(any(test, feature = "kittest"))]
#[must_use]
pub fn unsupported_style_warning_count_for(token: &'static str, id: NodeId) -> usize {
    let Some(warned) = WARNED_UNSUPPORTED.get() else {
        return 0;
    };
    let Ok(warned) = warned.lock() else {
        return 0;
    };
    usize::from(warned.contains(&(UnsupportedSlot::UnknownThemeRef(token), id)))
}

#[must_use]
pub fn render_style_for(
    id: NodeId,
    node: &Node,
    inherited: &Theme,
    inherited_text: Option<Rgba8>,
) -> RenderStyle {
    let mut render = RenderStyle::default();
    let theme = node.get_extension::<Theme>().unwrap_or(inherited);
    if matches!(node.kind, NodeType::Text { .. }) {
        render.text_color = Some(inherited_text.unwrap_or(theme.on_surface));
    }
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return render;
    };
    let tone = node
        .get_extension::<ToneContext>()
        .copied()
        .unwrap_or_default()
        .tone;
    let engine = StyleEngine::with_tone(theme, tone);

    for atom in binding.cascade() {
        apply_atoms(engine, &[atom], &mut render, id, node);
    }

    if let Some(overlay) = binding.animator_overlay() {
        StyleEngine::overlay_render(overlay, &mut render);
    }
    render
}

#[must_use]
pub fn text_style_for(node: &Node, inherited: &Theme, base: &TextStyle) -> TextStyle {
    let mut style = base.clone();
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return style;
    };

    let theme = node.get_extension::<Theme>().unwrap_or(inherited);
    let tone = node
        .get_extension::<ToneContext>()
        .copied()
        .unwrap_or_default()
        .tone;
    let engine = StyleEngine::with_tone(theme, tone);

    for atom in binding.cascade() {
        engine.apply_text(&[atom], &mut style);
    }
    if let Some(overlay) = binding.animator_overlay() {
        StyleEngine::overlay_text(overlay, &mut style);
    }
    style
}

fn apply_atoms(
    engine: StyleEngine<'_>,
    atoms: &[StyleAtom],
    render: &mut RenderStyle,
    id: NodeId,
    node: &Node,
) {
    engine.apply(atoms, render);
    if cfg!(debug_assertions) {
        for atom in atoms {
            if let StyleAtom::UnknownThemeRef(token) = atom {
                warn_unsupported(UnsupportedSlot::UnknownThemeRef(token), id, node);
            }
        }
    }
}

static WARNED_UNSUPPORTED: OnceLock<Mutex<HashSet<(UnsupportedSlot, NodeId)>>> = OnceLock::new();

fn warn_unsupported(slot: UnsupportedSlot, id: NodeId, node: &Node) {
    let warned = WARNED_UNSUPPORTED.get_or_init(|| Mutex::new(HashSet::new()));
    let Ok(mut warned) = warned.lock() else {
        return;
    };
    if !warned.insert((slot, id)) {
        return;
    }
    match slot {
        UnsupportedSlot::UnknownThemeRef(token) => {
            tracing::warn!(
                token,
                node = ?id,
                role = ?node.a11y.role,
                "unsupported style atom"
            );
        }
    }
}
