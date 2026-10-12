use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::animation_runtime::presets;
use crate::kernel::{NodeBuilder, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::Signal;
use crate::style::TransitionId;
use crate::style::{
    BORDER_WIDTH, BgGradient, ColorAtom, Elevation, GradientAxis, GradientStopKey, Rgba8,
    StateStyle, Style, StyleAtom, StyleVariant, Token,
};

use crate::widgets::interaction::InteractionState;

use super::super::shared::binary_style_state;
use super::builder::SliderRuntime;
use crate::widgets::orientation::Orientation;

const SLIDER_TRANSITION: TransitionId = presets::SMOOTH;
static SLIDER_THUMB_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(SLIDER_TRANSITION)];

const THUMB_SIZE: f32 = 16.0;
const TRACK_THICKNESS: f32 = 6.0;
const CELL_THUMB: &str = "●";

type ThumbForm = fn(InteractionState, Option<Signal<bool>>, bool) -> NodeBuilder;

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> ColorAtom {
    ColorAtom::Rgba8(Rgba8 { r, g, b, a })
}

const fn vertical_gradient(stops: &'static [GradientStopKey]) -> StyleAtom {
    StyleAtom::BackgroundGradient(BgGradient::new(GradientAxis::TopToBottom, stops))
}

const THUMB_BORDER: StyleAtom = StyleAtom::BorderWidth(BORDER_WIDTH);

static TRACK_REST: &[StyleAtom] = &[StyleAtom::BgToneStep(4)];
static TRACK_FILL: &[StyleAtom] = &[Token::Accent.bg()];
static CELL_THUMB_ATOMS: &[StyleAtom] = &[Token::Accent.text()];
static THUMB_REST: &[StyleAtom] = &[
    Token::OnAccent.bg(),
    THUMB_BORDER,
    Token::Accent.border(),
    StyleAtom::Shadow(Elevation::Sm),
];
static THUMB_HOVER: &[StyleAtom] = &[
    Token::OnAccent.bg(),
    THUMB_BORDER,
    Token::AccentHover.border(),
    StyleAtom::Shadow(Elevation::Sm),
];
static THUMB_DISABLED: &[StyleAtom] = &[Token::OnAccent.bg(), THUMB_BORDER, Token::Gray8.border()];

static GLASS_TRACK_STOPS: &[GradientStopKey] = &[
    GradientStopKey::new(0, rgba(148, 163, 184, 66)),
    GradientStopKey::new(500, rgba(148, 163, 184, 30)),
    GradientStopKey::new(1000, rgba(148, 163, 184, 46)),
];
static GLASS_FILL_STOPS: &[GradientStopKey] = &[
    GradientStopKey::new(0, Token::AccentHover.color()),
    GradientStopKey::new(1000, Token::Accent.color()),
];
static GLASS_THUMB_STOPS: &[GradientStopKey] = &[
    GradientStopKey::new(0, rgba(255, 255, 255, 250)),
    GradientStopKey::new(500, rgba(240, 243, 248, 235)),
    GradientStopKey::new(1000, rgba(214, 219, 230, 240)),
];
static GLASS_TRACK_REST: &[StyleAtom] = &[
    vertical_gradient(GLASS_TRACK_STOPS),
    THUMB_BORDER,
    StyleAtom::BorderColor(rgba(148, 163, 184, 90)),
];
static GLASS_TRACK_FILL: &[StyleAtom] = &[vertical_gradient(GLASS_FILL_STOPS)];
const GLASS_THUMB_BG: StyleAtom = vertical_gradient(GLASS_THUMB_STOPS);
static GLASS_THUMB_REST: &[StyleAtom] = &[
    GLASS_THUMB_BG,
    THUMB_BORDER,
    StyleAtom::BorderColor(rgba(255, 255, 255, 140)),
    StyleAtom::Shadow(Elevation::Md),
];
static GLASS_THUMB_HOVER: &[StyleAtom] = &[
    GLASS_THUMB_BG,
    THUMB_BORDER,
    StyleAtom::BorderColor(rgba(255, 255, 255, 190)),
    StyleAtom::Shadow(Elevation::Md),
];
static GLASS_THUMB_DISABLED: &[StyleAtom] = &[
    GLASS_THUMB_BG,
    THUMB_BORDER,
    StyleAtom::BorderColor(rgba(255, 255, 255, 60)),
];

pub(super) fn track(runtime: SliderRuntime, filled: bool) -> NodeBuilder {
    let layout = if filled {
        LayoutStyle::block()
    } else {
        LayoutStyle::block().grow(1.0)
    };
    let layout = match runtime.orientation {
        Orientation::Horizontal => layout.height(TRACK_THICKNESS),
        Orientation::Vertical => layout.width(TRACK_THICKNESS),
    };
    let class = match (filled, runtime.glass) {
        (true, true) => GLASS_TRACK_FILL,
        (true, false) => TRACK_FILL,
        (false, true) => GLASS_TRACK_REST,
        (false, false) => TRACK_REST,
    };
    let node = container()
        .hittable(false)
        .layout(layout)
        .style(Style::new().radius(3))
        .class(class);
    if filled {
        node.style_reactive(move || {
            let ratio = (runtime.value.get() - runtime.range.min) / runtime.range.span();
            cached_fill_atoms(runtime.orientation, ratio)
        })
    } else {
        node
    }
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
    map.entry((orientation == Orientation::Vertical, scaled))
        .or_insert_with(build)
}

pub(super) fn slider_thumb(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    glass: bool,
) -> NodeBuilder {
    CellForm::<ThumbForm>::new(window_thumb, cell_thumb).pick()(interaction, disabled, glass)
}

fn cell_thumb(
    _interaction: InteractionState,
    _disabled: Option<Signal<bool>>,
    _glass: bool,
) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(
            LayoutStyle::block()
                .width(cols(1.0))
                .height(rows(1.0))
                .shrink(0.0),
        )
        .class(CELL_THUMB_ATOMS)
        .child(text(CELL_THUMB))
}

fn window_thumb(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    glass: bool,
) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(THUMB_SIZE, THUMB_SIZE))
        .style(Style::new().radius(8))
        .class(SLIDER_THUMB_ANIM_BASE)
        .style_reactive(move || {
            slider_thumb_variant(glass).state(binary_style_state(interaction, disabled))
        })
}

const fn slider_thumb_variant(glass: bool) -> StyleVariant {
    if glass {
        StyleVariant::new(
            GLASS_THUMB_REST,
            StateStyle::new(
                GLASS_THUMB_REST,
                GLASS_THUMB_HOVER,
                GLASS_THUMB_HOVER,
                GLASS_THUMB_HOVER,
                GLASS_THUMB_DISABLED,
            ),
        )
    } else {
        StyleVariant::new(
            THUMB_REST,
            StateStyle::new(
                THUMB_REST,
                THUMB_HOVER,
                THUMB_HOVER,
                THUMB_HOVER,
                THUMB_DISABLED,
            ),
        )
    }
}
