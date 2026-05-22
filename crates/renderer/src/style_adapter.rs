//! Renderer-side style resolution.

#![allow(clippy::redundant_pub_crate)]

use std::sync::{Mutex, OnceLock};

use ahash::AHashSet;
use kernel::{Node, NodeId};
use style::{
    FocusRing, Px, RenderStyle, Rgba8, StyleAtom, StyleBinding, StyleEngine, TextStyle, Theme,
    ToneContext,
};

use crate::corner_radii::CornerRadii;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum UnsupportedSlot {
    UnknownThemeRef(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedFocusRing {
    pub(crate) color: Rgba8,
    pub(crate) width: f32,
    pub(crate) offset: f32,
    pub(crate) opacity: u16,
}

/// Return whether a given unsupported style warning has fired.
#[doc(hidden)]
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

pub(crate) fn render_style_for(id: NodeId, node: &Node, inherited: &Theme) -> RenderStyle {
    let mut render = RenderStyle::default();
    let theme = node.get_extension::<Theme>().unwrap_or(inherited);
    if matches!(node.kind, kernel::NodeType::Text { .. }) {
        render.text_color = Some(theme.on_surface);
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
    apply_atoms(engine, binding.reactive_slice(), &mut render, id, node);
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
        engine.apply_with_overlay(binding.reactive_slice(), overlay, &mut render);
        engine.apply_with_overlay(binding.state_overlay_atoms(), overlay, &mut render);
    }
    render
}

pub(crate) fn text_style_for(node: &Node, inherited: &Theme) -> TextStyle {
    let mut style = TextStyle::default();
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
    engine.apply_text(binding.reactive_slice(), &mut style);
    engine.apply_text(binding.state_overlay_atoms(), &mut style);
    style
}

pub(crate) fn focus_ring_for(node: &Node, inherited: &Theme) -> Option<ResolvedFocusRing> {
    let ring = node.get_extension::<FocusRing>()?;
    if !ring.visible() {
        return None;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherited);
    let engine = StyleEngine::new(theme);
    Some(ResolvedFocusRing {
        color: engine.resolve_color(ring.color_atom()),
        width: ring.stroke_width().max(0.0),
        offset: ring.offset_px().max(0.0),
        opacity: ring.opacity_value(),
    })
}

pub(crate) fn corner_radii_for(border_radius: Option<Px>) -> CornerRadii {
    border_radius.map_or(CornerRadii::ZERO, |radius| {
        CornerRadii::uniform(f32::from(radius.0))
    })
}

fn apply_atoms(
    engine: StyleEngine<'_>,
    atoms: &[StyleAtom],
    render: &mut RenderStyle,
    id: NodeId,
    node: &Node,
) {
    engine.apply(atoms, render);
    for atom in atoms {
        if let StyleAtom::UnknownThemeRef(token) = atom {
            warn_unsupported(UnsupportedSlot::UnknownThemeRef(token), id, node);
        }
    }
}

static WARNED_UNSUPPORTED: OnceLock<Mutex<AHashSet<(UnsupportedSlot, NodeId)>>> = OnceLock::new();

fn warn_unsupported(slot: UnsupportedSlot, id: NodeId, node: &Node) {
    let warned = WARNED_UNSUPPORTED.get_or_init(|| Mutex::new(AHashSet::new()));
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
