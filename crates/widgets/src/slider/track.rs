//! Slider visual layer — track line, value fill, and thumb nodes plus
//! their state-variant tables. Builder + interaction wiring live in
//! `builder.rs`; pointer/keyboard handling in `handlers.rs`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_animation_runtime::presets;
use sigil_kernel::{NodeBuilder, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{NodeStyleExt, Px, StateStyle, Style, StyleAtom, StyleVariant, TransitionId};

use crate::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::super::transition_seam::install_transition_animators;
use super::builder::{Orientation, SliderRuntime};

const SLIDER_TRANSITION: TransitionId = presets::SMOOTH;
const SLIDER_COMPONENT_ID: u64 = 0x5110_E000;
// Animator-only base slice: scanned by `install_transition_animators`
// for the `Transition` atom that flips the thumb into the animated path.
static SLIDER_THUMB_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(SLIDER_TRANSITION)];

const THUMB_SIZE: f32 = 16.0;
const TRACK_THICKNESS: f32 = 4.0;

// Rest-of-track uses `border-strong` (gray-8): quiet but still
// measurable against both step-1 and step-2 surfaces (the gauge/
// progress tracks at `surface-hover` measured ~1.3:1 — near-invisible,
// visual review 2026-08-15).
static TRACK_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border-strong")];
static TRACK_FILL: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-8")];
static ROOT_DISABLED: &[StyleAtom] = &[StyleAtom::Opacity(5_000)];
// The knob paints in `on-accent` (white) with a 1px step-7 rim so it
// separates from its own accent fill — an accent knob on the accent
// fill was only perceivable over the unfilled track (Fable 2026-08-15).
static THUMB_REST: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-accent"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderToneStep(7),
];
static THUMB_HOVER: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-accent"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderToneStep(8),
];
static THUMB_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("on-accent"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderToneStep(8),
];
static THUMB_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::Opacity(5_000),
];

fn value_ratio(runtime: SliderRuntime) -> f32 {
    ((runtime.value.get() - runtime.range.min) / runtime.range.span()).clamp(0.0, 1.0)
}

pub(super) fn track_fill(runtime: SliderRuntime) -> NodeBuilder {
    let layout = match runtime.orientation {
        Orientation::Horizontal => LayoutStyle::block().height(TRACK_THICKNESS),
        Orientation::Vertical => LayoutStyle::block().width(TRACK_THICKNESS),
    };
    container()
        // Inner parts are paint-only: the root owns all pointer handling,
        // and pointer capture binds to the deepest HITTABLE node — a
        // hittable fill/thumb/rest would steal capture and deafen the
        // root's drag handlers.
        .hittable(false)
        .layout(layout)
        // 2px radius keeps the line's ends soft; the fill's inner end is
        // always hidden behind the thumb.
        .style(Style::new().radius(2))
        .class(TRACK_FILL)
        .style_reactive(move || fill_extent_atoms(runtime))
}

fn fill_extent_atoms(runtime: SliderRuntime) -> &'static [StyleAtom] {
    let ratio = value_ratio(runtime);
    cached_fill_atoms(runtime.orientation, ratio)
}

// Leak-and-cache per distinct extent — the gauge fill pattern: a node
// carries one reactive atom slot returning `&'static [StyleAtom]`, so
// computed extents go through a bounded static map (per-mille steps).
type FillAtomCache = Mutex<HashMap<(bool, u16), &'static [StyleAtom]>>;

fn cached_fill_atoms(orientation: Orientation, ratio: f32) -> &'static [StyleAtom] {
    static CACHE: OnceLock<FillAtomCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let scaled = (ratio.clamp(0.0, 1.0) * 10_000.0).round() as u16;
    let key = (orientation == Orientation::Vertical, scaled);
    let build = || {
        let extent = sigil_style::Dimension::Fraction(scaled);
        let atom = match orientation {
            Orientation::Horizontal => StyleAtom::Width(extent),
            Orientation::Vertical => StyleAtom::Height(extent),
        };
        Box::leak(Box::new([atom])) as &'static [StyleAtom]
    };
    let Ok(mut map) = cache.lock() else {
        return build();
    };
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let leaked = build();
    map.insert(key, leaked);
    leaked
}

pub(super) fn track_rest(runtime: SliderRuntime) -> NodeBuilder {
    let layout = match runtime.orientation {
        Orientation::Horizontal => LayoutStyle::block().grow(1.0).height(TRACK_THICKNESS),
        Orientation::Vertical => LayoutStyle::block().grow(1.0).width(TRACK_THICKNESS),
    };
    container()
        .hittable(false)
        .layout(layout)
        .style(Style::new().radius(2))
        .class(TRACK_REST)
}

pub(super) fn slider_thumb(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> NodeBuilder {
    let state_style = slider_thumb_state_style();
    let node = container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(THUMB_SIZE, THUMB_SIZE))
        // THUMB is 16 px; explicit radius avoids the Px(9_999) SDF
        // clipping bug (radio_group precedent). Radius 8 = full circle.
        .style(Style::new().radius(8))
        .class(SLIDER_THUMB_ANIM_BASE)
        .style_reactive(move || {
            slider_thumb_variant().state(binary_style_state(interaction, disabled))
        });
    install_transition_animators(
        node,
        SLIDER_THUMB_ANIM_BASE,
        state_style,
        SLIDER_COMPONENT_ID,
        interaction,
    )
}

const fn slider_thumb_state_style() -> StateStyle {
    StateStyle::new(
        THUMB_REST,
        THUMB_HOVER,
        THUMB_HOVER,
        THUMB_DOWN,
        THUMB_DISABLED,
    )
}

pub(super) const fn slider_root_variant() -> StyleVariant {
    // Root is the transparent hit area; disabled dims the whole control.
    const EMPTY: &[StyleAtom] = &[];
    StyleVariant::new(
        EMPTY,
        StateStyle::new(EMPTY, EMPTY, EMPTY, EMPTY, ROOT_DISABLED),
    )
}

const fn slider_thumb_variant() -> StyleVariant {
    StyleVariant::new(
        THUMB_REST,
        StateStyle::new(
            THUMB_REST,
            THUMB_HOVER,
            THUMB_HOVER,
            THUMB_DOWN,
            THUMB_DISABLED,
        ),
    )
}
