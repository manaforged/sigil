//! Renderer-side style resolution.

#![allow(clippy::redundant_pub_crate)]

use std::sync::{Mutex, OnceLock};

use ahash::AHashSet;
use sigil_kernel::{Node, NodeId};
use sigil_style::{
    FocusRing, Px, RenderStyle, Rgba8, StyleAtom, StyleBinding, StyleEngine, TextStyle, Theme,
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

pub(crate) fn render_style_for(id: NodeId, node: &Node) -> RenderStyle {
    let mut render = RenderStyle::default();
    if matches!(node.kind, sigil_kernel::NodeType::Text { .. }) {
        render.text_color = Some(default_theme().on_surface);
    }
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return render;
    };

    let theme = node.get_extension::<Theme>().unwrap_or_else(default_theme);
    let engine = StyleEngine::new(theme);

    apply_atoms(engine, binding.static_slice(), &mut render, id, node);
    apply_atoms(engine, binding.dynamic_slice(), &mut render, id, node);
    apply_atoms(engine, binding.reactive_slice(), &mut render, id, node);
    render
}

pub(crate) fn text_style_for(node: &Node) -> TextStyle {
    let mut style = TextStyle::default();
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return style;
    };

    let theme = node.get_extension::<Theme>().unwrap_or_else(default_theme);
    let engine = StyleEngine::new(theme);

    engine.apply_text(binding.static_slice(), &mut style);
    engine.apply_text(binding.dynamic_slice(), &mut style);
    engine.apply_text(binding.reactive_slice(), &mut style);
    style
}

pub(crate) fn focus_ring_for(node: &Node) -> Option<ResolvedFocusRing> {
    let ring = node.get_extension::<FocusRing>()?;
    if !ring.visible() {
        return None;
    }
    let theme = node.get_extension::<Theme>().unwrap_or_else(default_theme);
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

fn default_theme<'theme>() -> &'theme Theme {
    static DEFAULT_THEME: OnceLock<Theme> = OnceLock::new();
    DEFAULT_THEME.get_or_init(Theme::default)
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
