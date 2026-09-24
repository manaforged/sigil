use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::animation_runtime::presets;
use crate::kernel::{NodeBuilder, container};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{
    BORDER_WIDTH, Elevation, NodeStyleExt, StateStyle, Style, StyleAtom, StyleVariant, Token,
    TransitionId,
};

use crate::widgets::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::builder::{Orientation, SliderRuntime};

const SLIDER_TRANSITION: TransitionId = presets::SMOOTH;
static SLIDER_THUMB_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(SLIDER_TRANSITION)];

const THUMB_SIZE: f32 = 16.0;
const TRACK_THICKNESS: f32 = 6.0;

static TRACK_REST: &[StyleAtom] = &[StyleAtom::BgToneStep(4)];
static TRACK_FILL: &[StyleAtom] = &[Token::Accent.bg()];
static THUMB_REST: &[StyleAtom] = &[
    Token::OnAccent.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Accent.border(),
    StyleAtom::Shadow(Elevation::Sm),
];
static THUMB_HOVER: &[StyleAtom] = &[
    Token::OnAccent.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::AccentHover.border(),
    StyleAtom::Shadow(Elevation::Sm),
];
static THUMB_DOWN: &[StyleAtom] = &[
    Token::OnAccent.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::AccentHover.border(),
    StyleAtom::Shadow(Elevation::Sm),
];
static THUMB_DISABLED: &[StyleAtom] = &[
    Token::OnAccent.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Gray8.border(),
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
        .hittable(false)
        .layout(layout)
        .style(Style::new().radius(3))
        .class(TRACK_FILL)
        .style_reactive(move || fill_extent_atoms(runtime))
}

fn fill_extent_atoms(runtime: SliderRuntime) -> &'static [StyleAtom] {
    let ratio = value_ratio(runtime);
    cached_fill_atoms(runtime.orientation, ratio)
}

type FillAtomCache = Mutex<HashMap<(bool, u16), &'static [StyleAtom]>>;

fn cached_fill_atoms(orientation: Orientation, ratio: f32) -> &'static [StyleAtom] {
    static CACHE: OnceLock<FillAtomCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let scaled = (ratio.clamp(0.0, 1.0) * 10_000.0).round() as u16;
    let key = (orientation == Orientation::Vertical, scaled);
    let build = || {
        let extent = crate::style::Dimension::Fraction(scaled);
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
        .style(Style::new().radius(3))
        .class(TRACK_REST)
}

pub(super) fn slider_thumb(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(THUMB_SIZE, THUMB_SIZE))
        .style(Style::new().radius(8))
        .class(SLIDER_THUMB_ANIM_BASE)
        .style_reactive(move || {
            slider_thumb_variant().state(binary_style_state(interaction, disabled))
        })
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
