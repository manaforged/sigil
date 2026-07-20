//! Class-list merge support.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use smallvec::SmallVec;

use crate::{Px, StyleAtom};

/// Stable component identifier used for merge diagnostics.
pub type ComponentId = u64;

/// Merge default atoms with override atoms, resolving conflicts last-wins.
#[must_use]
pub fn merge_classes(
    defaults: &[StyleAtom],
    overrides: &[StyleAtom],
    component_id: ComponentId,
) -> SmallVec<[StyleAtom; 32]> {
    let mut merged = SmallVec::<[(ResolvedSlot, StyleAtom); 32]>::new();
    for atom in defaults {
        insert_atom(&mut merged, *atom, component_id);
    }
    for atom in overrides {
        insert_atom(&mut merged, *atom, component_id);
    }
    merged.into_iter().map(|(_slot, atom)| atom).collect()
}

/// Return the number of merge conflicts warned in this session.
#[must_use]
pub fn merge_warning_count() -> usize {
    diagnostic::warning_count()
}

/// Return merge conflicts warned for a specific component.
#[must_use]
pub fn merge_warning_count_for_component(component_id: ComponentId) -> usize {
    diagnostic::warning_count_for_component(component_id)
}

/// Number of unknown-class tokens dropped by the class parser this session.
///
/// Bumped by [`bump_unknown_class_count`] when `.class("...")` receives a
/// token outside the documented atomic-class catalog. Useful in tests to
/// assert the catalog covers every token a widget emits.
#[must_use]
pub fn unknown_class_warning_count() -> usize {
    diagnostic::unknown_class_count()
}

/// Bump the unknown-class counter. Called from the class parser's diagnostic
/// path alongside the `tracing::warn!` emission.
pub fn bump_unknown_class_count() {
    diagnostic::bump_unknown_class();
}

fn insert_atom(
    merged: &mut SmallVec<[(ResolvedSlot, StyleAtom); 32]>,
    atom: StyleAtom,
    component_id: ComponentId,
) {
    let mut expanded = SmallVec::<[(ResolvedSlot, StyleAtom); 4]>::new();
    expand_atom(atom, &mut expanded);
    for (slot, expanded_atom) in expanded {
        if let Some((_existing_slot, existing_atom)) = merged
            .iter_mut()
            .find(|(existing, _atom)| *existing == slot)
        {
            if *existing_atom != expanded_atom {
                diagnostic::warn_conflict(slot, component_id);
            }
            *existing_atom = expanded_atom;
        } else {
            merged.push((slot, expanded_atom));
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive match on StyleAtom — the long no-op arm chain is intentional; a wildcard would silently swallow new atom kinds"
)]
fn expand_atom(atom: StyleAtom, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    match atom {
        StyleAtom::PaddingAll(px) => expand_padding_all(px, out),
        StyleAtom::PaddingX(px) => expand_padding_x(px, out),
        StyleAtom::PaddingY(px) => expand_padding_y(px, out),
        StyleAtom::MarginAll(px) => expand_margin_all(px, out),
        StyleAtom::MarginX(px) => expand_margin_x(px, out),
        StyleAtom::MarginY(px) => expand_margin_y(px, out),
        StyleAtom::PaddingTop(_)
        | StyleAtom::PaddingRight(_)
        | StyleAtom::PaddingBottom(_)
        | StyleAtom::PaddingLeft(_)
        | StyleAtom::MarginTop(_)
        | StyleAtom::MarginRight(_)
        | StyleAtom::MarginBottom(_)
        | StyleAtom::MarginLeft(_)
        | StyleAtom::Gap(_)
        | StyleAtom::GapX(_)
        | StyleAtom::GapY(_)
        | StyleAtom::Width(_)
        | StyleAtom::Height(_)
        | StyleAtom::MinWidth(_)
        | StyleAtom::MaxWidth(_)
        | StyleAtom::MinHeight(_)
        | StyleAtom::MaxHeight(_)
        | StyleAtom::Display(_)
        | StyleAtom::FlexDirection(_)
        | StyleAtom::FlexWrap(_)
        | StyleAtom::AlignItems(_)
        | StyleAtom::JustifyContent(_)
        | StyleAtom::FlexGrow(_)
        | StyleAtom::FlexShrink(_)
        | StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::BorderRadius(_)
        | StyleAtom::BorderWidth(_)
        | StyleAtom::BorderColor(_)
        | StyleAtom::BorderThemeRef(_)
        | StyleAtom::BorderToneStep(_)
        | StyleAtom::Opacity(_)
        | StyleAtom::FontSize(_)
        | StyleAtom::FontWeight(_)
        | StyleAtom::FontFamily(_)
        | StyleAtom::LineHeight(_)
        | StyleAtom::OverflowX(_)
        | StyleAtom::OverflowY(_)
        | StyleAtom::Transition(_)
        | StyleAtom::TransitionDuration(_)
        | StyleAtom::UnknownThemeRef(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => out.push((slot(atom), atom)),
    }
}

fn expand_padding_all(px: Px, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    out.extend([
        (ResolvedSlot::PaddingTop, StyleAtom::PaddingTop(px)),
        (ResolvedSlot::PaddingRight, StyleAtom::PaddingRight(px)),
        (ResolvedSlot::PaddingBottom, StyleAtom::PaddingBottom(px)),
        (ResolvedSlot::PaddingLeft, StyleAtom::PaddingLeft(px)),
    ]);
}

fn expand_padding_x(px: Px, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    out.extend([
        (ResolvedSlot::PaddingRight, StyleAtom::PaddingRight(px)),
        (ResolvedSlot::PaddingLeft, StyleAtom::PaddingLeft(px)),
    ]);
}

fn expand_padding_y(px: Px, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    out.extend([
        (ResolvedSlot::PaddingTop, StyleAtom::PaddingTop(px)),
        (ResolvedSlot::PaddingBottom, StyleAtom::PaddingBottom(px)),
    ]);
}

fn expand_margin_all(px: Px, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    out.extend([
        (ResolvedSlot::MarginTop, StyleAtom::MarginTop(px)),
        (ResolvedSlot::MarginRight, StyleAtom::MarginRight(px)),
        (ResolvedSlot::MarginBottom, StyleAtom::MarginBottom(px)),
        (ResolvedSlot::MarginLeft, StyleAtom::MarginLeft(px)),
    ]);
}

fn expand_margin_x(px: Px, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    out.extend([
        (ResolvedSlot::MarginRight, StyleAtom::MarginRight(px)),
        (ResolvedSlot::MarginLeft, StyleAtom::MarginLeft(px)),
    ]);
}

fn expand_margin_y(px: Px, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    out.extend([
        (ResolvedSlot::MarginTop, StyleAtom::MarginTop(px)),
        (ResolvedSlot::MarginBottom, StyleAtom::MarginBottom(px)),
    ]);
}

#[allow(clippy::too_many_lines)]
const fn slot(atom: StyleAtom) -> ResolvedSlot {
    match atom {
        StyleAtom::PaddingAll(_) | StyleAtom::PaddingY(_) | StyleAtom::PaddingTop(_) => {
            ResolvedSlot::PaddingTop
        }
        StyleAtom::PaddingX(_) | StyleAtom::PaddingRight(_) => ResolvedSlot::PaddingRight,
        StyleAtom::PaddingBottom(_) => ResolvedSlot::PaddingBottom,
        StyleAtom::PaddingLeft(_) => ResolvedSlot::PaddingLeft,
        StyleAtom::MarginAll(_) | StyleAtom::MarginY(_) | StyleAtom::MarginTop(_) => {
            ResolvedSlot::MarginTop
        }
        StyleAtom::MarginX(_) | StyleAtom::MarginRight(_) => ResolvedSlot::MarginRight,
        StyleAtom::MarginBottom(_) => ResolvedSlot::MarginBottom,
        StyleAtom::MarginLeft(_) => ResolvedSlot::MarginLeft,
        StyleAtom::Gap(_) => ResolvedSlot::Gap,
        StyleAtom::GapX(_) => ResolvedSlot::GapX,
        StyleAtom::GapY(_) => ResolvedSlot::GapY,
        StyleAtom::Width(_) => ResolvedSlot::Width,
        StyleAtom::Height(_) => ResolvedSlot::Height,
        StyleAtom::MinWidth(_) => ResolvedSlot::MinWidth,
        StyleAtom::MaxWidth(_) => ResolvedSlot::MaxWidth,
        StyleAtom::MinHeight(_) => ResolvedSlot::MinHeight,
        StyleAtom::MaxHeight(_) => ResolvedSlot::MaxHeight,
        StyleAtom::Display(_) => ResolvedSlot::Display,
        StyleAtom::FlexDirection(_) => ResolvedSlot::FlexDirection,
        StyleAtom::FlexWrap(_) => ResolvedSlot::FlexWrap,
        StyleAtom::AlignItems(_) => ResolvedSlot::AlignItems,
        StyleAtom::JustifyContent(_) => ResolvedSlot::JustifyContent,
        StyleAtom::FlexGrow(_) => ResolvedSlot::FlexGrow,
        StyleAtom::FlexShrink(_) => ResolvedSlot::FlexShrink,
        StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::BgToneStep(_) => ResolvedSlot::BackgroundColor,
        StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => ResolvedSlot::TextColor,
        StyleAtom::BorderRadius(_) => ResolvedSlot::BorderRadius,
        StyleAtom::BorderWidth(_) => ResolvedSlot::BorderWidth,
        StyleAtom::BorderColor(_) | StyleAtom::BorderThemeRef(_) | StyleAtom::BorderToneStep(_) => {
            ResolvedSlot::BorderColor
        }
        StyleAtom::Opacity(_) => ResolvedSlot::Opacity,
        StyleAtom::FontSize(_) => ResolvedSlot::FontSize,
        StyleAtom::FontWeight(_) => ResolvedSlot::FontWeight,
        StyleAtom::FontFamily(_) => ResolvedSlot::FontFamily,
        StyleAtom::LineHeight(_) => ResolvedSlot::LineHeight,
        StyleAtom::OverflowX(_) => ResolvedSlot::OverflowX,
        StyleAtom::OverflowY(_) => ResolvedSlot::OverflowY,
        StyleAtom::Transition(_) => ResolvedSlot::Transition,
        StyleAtom::TransitionDuration(_) => ResolvedSlot::TransitionDuration,
        StyleAtom::UnknownThemeRef(_) => ResolvedSlot::UnknownThemeRef,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ResolvedSlot {
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Gap,
    GapX,
    GapY,
    Width,
    Height,
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
    Display,
    FlexDirection,
    FlexWrap,
    AlignItems,
    JustifyContent,
    FlexGrow,
    FlexShrink,
    BackgroundColor,
    TextColor,
    BorderRadius,
    BorderWidth,
    BorderColor,
    Opacity,
    FontSize,
    FontWeight,
    FontFamily,
    LineHeight,
    OverflowX,
    OverflowY,
    Transition,
    TransitionDuration,
    UnknownThemeRef,
}

mod diagnostic {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{ComponentId, HashSet, Mutex, OnceLock, ResolvedSlot};

    static WARNED: OnceLock<Mutex<HashSet<(ResolvedSlot, ComponentId)>>> = OnceLock::new();
    static UNKNOWN_CLASS: AtomicUsize = AtomicUsize::new(0);

    pub(super) fn bump_unknown_class() {
        UNKNOWN_CLASS.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn unknown_class_count() -> usize {
        UNKNOWN_CLASS.load(Ordering::Relaxed)
    }

    pub(super) fn warn_conflict(slot: ResolvedSlot, component_id: ComponentId) {
        let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
        let Ok(mut warned) = warned.lock() else {
            return;
        };
        if warned.insert((slot, component_id)) {
            eprintln!(
                "sigil-style merge conflict for component {component_id} slot {slot:?}; override wins",
            );
        }
    }

    pub(super) fn warning_count() -> usize {
        WARNED
            .get()
            .and_then(|warned| warned.lock().ok().map(|warned| warned.len()))
            .unwrap_or(0)
    }

    pub(super) fn warning_count_for_component(component_id: ComponentId) -> usize {
        WARNED
            .get()
            .and_then(|warned| {
                warned.lock().ok().map(|warned| {
                    warned
                        .iter()
                        .filter(|(_slot, warned_component)| *warned_component == component_id)
                        .count()
                })
            })
            .unwrap_or(0)
    }
}
