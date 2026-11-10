use crate::layout::Space;
use std::rc::Rc;

mod press;

use crate::kernel::{AccessibleName, NodeBuilder, TextContent};
use crate::layout::LayoutStyle;
use crate::reactive::{Signal, derived};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{
    BORDER_WIDTH, DISABLED_OPACITY, ICON_SIZE, Radius, StateStyle, Style, StyleAtom, StyleState,
    StyleVariant, Token, space,
};

use crate::widgets::interaction::InteractionState;

pub(crate) use press::{RELEASE, RELEASE_ENTER, RELEASE_ENTER_SPACE, flip, press};

pub(crate) type Activate = Rc<dyn Fn() + 'static>;
pub(super) type BoolChange = Rc<dyn Fn(bool) + 'static>;
pub(super) type TextChange = Rc<dyn Fn(&str) + 'static>;

pub(crate) fn noop_activate() -> Activate {
    Rc::new(|| {})
}

pub(crate) fn binary_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let mut state = interaction.style_state();
    state.disabled |= disabled_now(disabled);
    state
}

pub(crate) fn reactive_style_state(
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
) -> StyleState {
    let _ = disabled.map(Signal::get);
    binary_style_state(interaction, disabled)
}

pub(crate) use crate::style::DIMMED;
static UNDIMMED: &[StyleAtom] = &[];
pub(crate) static ICON_GLYPH: &[StyleAtom] = &[StyleAtom::FontSize(ICON_SIZE)];
pub(crate) static METER_TRACK: &[StyleAtom] = &[Radius::Md.atom(), StyleAtom::BgToneStep(4)];
pub(crate) const METER_TRACK_HEIGHT: f32 = 8.0;

static CONTROL_BASE: &[StyleAtom] = &[Radius::Md.atom()];
static CONTROL_HOVER: &[StyleAtom] = &[Token::SurfaceHover.background()];
static CONTROL_PRESSED: &[StyleAtom] = &[Token::SurfacePressed.background()];
static CONTROL_VARIANT: StyleVariant = StyleVariant::new(
    UNDIMMED,
    StateStyle::new(UNDIMMED, UNDIMMED, CONTROL_HOVER, CONTROL_PRESSED, DIMMED),
);

static FIELD_BASE: &[StyleAtom] = &[
    Radius::Md.atom(),
    Token::Field.background(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
];
static FIELD_REST: &[StyleAtom] = &[StyleAtom::BorderToneStep(7)];
static FIELD_HOVER: &[StyleAtom] = &[StyleAtom::BorderToneStep(8)];
static FIELD_ACTIVE: &[StyleAtom] = &[Token::Accent.border()];
static FIELD_DISABLED: &[StyleAtom] = &[
    StyleAtom::BorderToneStep(6),
    StyleAtom::Opacity(DISABLED_OPACITY),
];
pub(crate) static FIELD_VARIANT: StyleVariant = StyleVariant::new(
    FIELD_BASE,
    StateStyle::new(
        FIELD_REST,
        FIELD_ACTIVE,
        FIELD_HOVER,
        FIELD_ACTIVE,
        FIELD_DISABLED,
    ),
);

pub(crate) const CONTROL_INSET: f32 = space(1);

pub(crate) fn control_row(layout: LayoutStyle) -> LayoutStyle {
    layout.padding_x(Space::px(CONTROL_INSET))
}

pub(crate) fn control(node: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    node.class(CONTROL_BASE)
        .variant(CONTROL_VARIANT)
        .disabled_by(disabled)
}

pub(crate) fn disabled_a11y(node: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    match disabled {
        Some(flag) => node.mutate(move |node| node.a11y.disabled = Some(flag)),
        None => node,
    }
}

pub(crate) fn disabled_look(node: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let Some(flag) = disabled else {
        return node;
    };
    disabled_a11y(node, disabled).style_reactive(move || if flag.get() { DIMMED } else { UNDIMMED })
}

pub(crate) fn disabled_now(disabled: Option<Signal<bool>>) -> bool {
    disabled.is_some_and(Signal::peek)
}

pub(crate) fn or_disabled(
    a: Option<Signal<bool>>,
    b: Option<Signal<bool>>,
) -> Option<Signal<bool>> {
    match (a, b) {
        (Some(left), Some(right)) => Some(derived(move || left.get() || right.get())),
        (left, None) => left,
        (None, right) => right,
    }
}

pub(super) fn noop_bool_change() -> BoolChange {
    Rc::new(|_| {})
}

pub(crate) fn accessible_name_or(
    label: Option<&TextContent>,
    fallback: StringKey,
) -> AccessibleName {
    label.map_or_else(
        || AccessibleName::Derived(theme_label(fallback)),
        accessible_name,
    )
}

pub(crate) fn accessible_name(label: &TextContent) -> AccessibleName {
    label.live().map_or_else(
        || AccessibleName::Static(label.resolve()),
        AccessibleName::Derived,
    )
}

pub(crate) fn caller_style(node: NodeBuilder, style: Style) -> NodeBuilder {
    if style.is_empty() {
        node
    } else {
        node.style(style)
    }
}
