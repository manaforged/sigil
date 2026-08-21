//! Resolve a node's style into paint values.
//!
//! Both hosts land here: the GPU renderer turns the result into draw
//! commands, the cell host turns it into colours and attributes. Resolving
//! in one place is what makes a terminal frame and a window frame agree on
//! what a node looks like.

use sigil_kernel::{Node, NodeId, NodeType};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::{
    RenderStyle, Rgba8, StyleAtom, StyleBinding, StyleEngine, TextStyle, Theme, ToneContext,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum UnsupportedSlot {
    UnknownThemeRef(&'static str),
}

/// Return whether a given unsupported style warning has fired.
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

/// Resolve `node`'s paint values against its theme and inherited text colour.
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
        // CSS-style color cascade: a bare text node paints the nearest
        // ancestor's resolved text color (a container's `text-*` class,
        // including reactive `class_when` atoms), falling back to the
        // theme's on-surface at the root. Before this, container text
        // classes never reached child glyphs — chip tones only looked
        // right because on-surface == gray-12 in the light theme.
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
    // DS2 Phase 2: per-instance user state overlay applies last so
    // last-atom-wins gives user `.hover()` / `.pressed()` / `.focus()`
    // / `.disabled()` atoms priority over widget-baked atoms.
    apply_atoms(engine, binding.state_overlay_atoms(), &mut render, id, node);

    // W5.1b: replay the same atom set through the animator overlay so
    // any driven numeric value wins over the literal atom value. The
    // unknown-theme-ref warning has already been emitted by the
    // literal pass above; this pass only updates the resolved render
    // values.
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

/// Resolve `node`'s typography, layered over the inherited `base`.
#[must_use]
pub fn text_style_for(node: &Node, inherited: &Theme, base: &TextStyle) -> TextStyle {
    // DR-056: seed from the inherited typography carried down the scene
    // walk, mirroring the layout engine's measurement resolution, then
    // layer this node's own atoms (own-wins).
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
    // Second pass over the same slice, for a development diagnostic. Resolve
    // runs for every node of every frame, so a release build does not pay for
    // it — an unknown token is an authoring mistake, caught while authoring.
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
