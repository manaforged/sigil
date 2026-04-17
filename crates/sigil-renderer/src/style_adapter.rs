//! Renderer-side style resolution.

#![allow(clippy::redundant_pub_crate)]

use std::sync::{Mutex, OnceLock};

use ahash::AHashSet;
use sigil_core::{Node, NodeId};
use sigil_style::{RenderStyle, StyleAtom, StyleBinding, StyleEngine, Theme};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum UnsupportedSlot {
    UnknownThemeRef(&'static str),
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
