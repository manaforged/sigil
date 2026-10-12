use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use smallvec::SmallVec;

use crate::style::{Px, StyleAtom};

pub type ComponentId = u64;

mod slots;
pub(crate) use slots::SlotSet;

#[must_use]
pub(crate) fn merge_classes(
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

static WARNED: OnceLock<Mutex<HashSet<(ResolvedSlot, ComponentId)>>> = OnceLock::new();
static UNKNOWN_CLASS: AtomicUsize = AtomicUsize::new(0);

#[must_use]
pub(crate) fn merge_warning_count_for_component(component_id: ComponentId) -> usize {
    WARNED
        .get()
        .and_then(|warned| warned.lock().ok())
        .map_or(0, |warned| {
            warned
                .iter()
                .filter(|(_slot, warned_component)| *warned_component == component_id)
                .count()
        })
}

#[must_use]
pub fn unknown_class_warning_count() -> usize {
    UNKNOWN_CLASS.load(Ordering::Relaxed)
}

pub(crate) fn bump_unknown_class_count() {
    UNKNOWN_CLASS.fetch_add(1, Ordering::Relaxed);
}

fn warn_conflict(slot: ResolvedSlot, component_id: ComponentId) {
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
            if *existing_atom != expanded_atom && component_id != 0 {
                warn_conflict(slot, component_id);
            }
            *existing_atom = expanded_atom;
        } else {
            merged.push((slot, expanded_atom));
        }
    }
}

fn expand_atom(atom: StyleAtom, out: &mut SmallVec<[(ResolvedSlot, StyleAtom); 4]>) {
    use ResolvedSlot as S;
    use StyleAtom as A;
    let (px, sides): (Px, &[S]) = match atom {
        A::PaddingAll(px) => (
            px,
            &[
                S::PaddingTop,
                S::PaddingRight,
                S::PaddingBottom,
                S::PaddingLeft,
            ],
        ),
        A::PaddingX(px) => (px, &[S::PaddingRight, S::PaddingLeft]),
        A::PaddingY(px) => (px, &[S::PaddingTop, S::PaddingBottom]),
        A::MarginAll(px) => (
            px,
            &[S::MarginTop, S::MarginRight, S::MarginBottom, S::MarginLeft],
        ),
        A::MarginX(px) => (px, &[S::MarginRight, S::MarginLeft]),
        A::MarginY(px) => (px, &[S::MarginTop, S::MarginBottom]),
        _ => return out.push((slot(atom), atom)),
    };
    out.extend(sides.iter().map(|&side| {
        let expanded = match side {
            S::PaddingTop => A::PaddingTop(px),
            S::PaddingRight => A::PaddingRight(px),
            S::PaddingBottom => A::PaddingBottom(px),
            S::PaddingLeft => A::PaddingLeft(px),
            S::MarginTop => A::MarginTop(px),
            S::MarginRight => A::MarginRight(px),
            S::MarginBottom => A::MarginBottom(px),
            _ => A::MarginLeft(px),
        };
        (side, expanded)
    }));
}

#[allow(
    clippy::too_many_lines,
    reason = "one sequential pass; splitting would scatter the ordering it documents"
)]
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
        StyleAtom::GridColumns(_) => ResolvedSlot::GridColumns,
        StyleAtom::GridRows(_) => ResolvedSlot::GridRows,
        StyleAtom::BackgroundColor(_)
        | StyleAtom::BackgroundThemeRef(_)
        | StyleAtom::BgToneStep(_)
        | StyleAtom::BackgroundGradient(_) => ResolvedSlot::BackgroundColor,
        StyleAtom::TextColor(_)
        | StyleAtom::TextThemeRef(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid => ResolvedSlot::TextColor,
        StyleAtom::BorderRadius(_) | StyleAtom::Radius(_) => ResolvedSlot::BorderRadius,
        StyleAtom::BorderWidth(_) => ResolvedSlot::BorderWidth,
        StyleAtom::BorderTop(_) => ResolvedSlot::BorderTop,
        StyleAtom::BorderRight(_) => ResolvedSlot::BorderRight,
        StyleAtom::BorderBottom(_) => ResolvedSlot::BorderBottom,
        StyleAtom::BorderLeft(_) => ResolvedSlot::BorderLeft,
        StyleAtom::BorderColor(_) | StyleAtom::BorderThemeRef(_) | StyleAtom::BorderToneStep(_) => {
            ResolvedSlot::BorderColor
        }
        StyleAtom::Shadow(_) => ResolvedSlot::Shadow,
        StyleAtom::Opacity(_) => ResolvedSlot::Opacity,
        StyleAtom::Transform(_) => ResolvedSlot::Transform,
        StyleAtom::FontSize(_) => ResolvedSlot::FontSize,
        StyleAtom::FontWeight(_) => ResolvedSlot::FontWeight,
        StyleAtom::FontItalic(_) => ResolvedSlot::FontItalic,
        StyleAtom::FontFamily(_) => ResolvedSlot::FontFamily,
        StyleAtom::LineHeight(_) => ResolvedSlot::LineHeight,
        StyleAtom::LetterSpacing(_) => ResolvedSlot::LetterSpacing,
        StyleAtom::TabularNums(_) => ResolvedSlot::TabularNums,
        StyleAtom::TextDecoration(_) => ResolvedSlot::TextDecoration,
        StyleAtom::OverflowX(_) => ResolvedSlot::OverflowX,
        StyleAtom::OverflowY(_) => ResolvedSlot::OverflowY,
        StyleAtom::Transition(_) => ResolvedSlot::Transition,
        StyleAtom::TransitionDuration(_) => ResolvedSlot::TransitionDuration,
        StyleAtom::UnknownThemeRef(_) => ResolvedSlot::UnknownThemeRef,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ResolvedSlot {
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
    GridColumns,
    GridRows,
    BackgroundColor,
    TextColor,
    BorderRadius,
    BorderWidth,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderColor,
    Shadow,
    Opacity,
    Transform,
    FontSize,
    FontWeight,
    FontItalic,
    FontFamily,
    LineHeight,
    LetterSpacing,
    TabularNums,
    TextDecoration,
    OverflowX,
    OverflowY,
    Transition,
    TransitionDuration,
    UnknownThemeRef,
}
