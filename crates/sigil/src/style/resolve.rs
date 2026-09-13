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

    apply_atoms(engine, binding.static_slice(), &mut render, id, node);
    apply_atoms(engine, binding.dynamic_slice(), &mut render, id, node);
    for reactive in binding.reactive_channels() {
        apply_atoms(engine, reactive, &mut render, id, node);
    }
    apply_atoms(engine, binding.state_overlay_atoms(), &mut render, id, node);

    if let Some(overlay) = binding.animator_overlay() {
        engine.apply_with_overlay(binding.static_slice(), overlay, &mut render);
        engine.apply_with_overlay(binding.dynamic_slice(), overlay, &mut render);
        for reactive in binding.reactive_channels() {
            engine.apply_with_overlay(reactive, overlay, &mut render);
        }
        engine.apply_with_overlay(binding.state_overlay_atoms(), overlay, &mut render);
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

    engine.apply_text(binding.static_slice(), &mut style);
    engine.apply_text(binding.dynamic_slice(), &mut style);
    for reactive in binding.reactive_channels() {
        engine.apply_text(reactive, &mut style);
    }
    engine.apply_text(binding.state_overlay_atoms(), &mut style);
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
