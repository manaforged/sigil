use std::rc::Rc;

use crate::events::{KeyEvent, PointerEvent};
use crate::kernel::NodeBuilder;
use crate::layout::LayoutStyle;
use crate::reactive::{Signal, derived};
use crate::style::{
    BORDER_WIDTH, DISABLED_OPACITY, ICON_SIZE, Radius, StateStyle, StyleAtom, StyleOverlay,
    StyleState, StyleVariant, Token, space,
};

use crate::widgets::interaction::InteractionState;

pub(super) type Activate = Rc<dyn Fn() + 'static>;
pub(super) type BoolChange = Rc<dyn Fn(bool) + 'static>;
pub(super) type BuilderMutator = Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>;

pub(super) fn activate_pointer(
    event: &mut PointerEvent,
    interaction: InteractionState,
    activate: &Activate,
) {
    if interaction.release_pointer(event) {
        activate();
    }
}

pub(super) fn activate_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    activate: &Activate,
) {
    if interaction.activate_key(event) {
        activate();
    }
}

pub(super) fn activate_pointer_reactive(
    event: &mut PointerEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    activate: &Activate,
) {
    if interaction.release_pointer(event) && !disabled_now(disabled) {
        activate();
    }
}

pub(super) fn activate_key_reactive(
    event: &mut KeyEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    activate: &Activate,
) {
    if interaction.activate_key(event) && !disabled_now(disabled) {
        activate();
    }
}

pub(crate) fn noop_activate() -> Activate {
    Rc::new(|| {})
}

pub(super) fn activate_binary_pointer(
    event: &mut PointerEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    value: Signal<bool>,
    on_change: &BoolChange,
) {
    let valid_release = interaction.release_pointer(event);
    if valid_release && !disabled_now(disabled) {
        toggle_bool(value, on_change);
    }
}

pub(super) fn activate_binary_key(
    event: &mut KeyEvent,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    value: Signal<bool>,
    on_change: &BoolChange,
) {
    if interaction.activate_key(event) && !disabled_now(disabled) {
        toggle_bool(value, on_change);
    }
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

pub(crate) static DIMMED: &[StyleAtom] = &[StyleAtom::Opacity(DISABLED_OPACITY)];
static UNDIMMED: &[StyleAtom] = &[];
pub(crate) static ICON_GLYPH: &[StyleAtom] = &[StyleAtom::FontSize(ICON_SIZE)];

static CONTROL_BASE: &[StyleAtom] = &[Radius::Md.atom()];
static CONTROL_HOVER: &[StyleAtom] = &[Token::SurfaceHover.bg()];
static CONTROL_PRESSED: &[StyleAtom] = &[Token::SurfacePressed.bg()];
static CONTROL_VARIANT: StyleVariant = StyleVariant::new(
    UNDIMMED,
    StateStyle::new(UNDIMMED, UNDIMMED, CONTROL_HOVER, CONTROL_PRESSED, DIMMED),
);

static FIELD_BASE: &[StyleAtom] = &[
    Radius::Md.atom(),
    Token::Field.bg(),
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
    layout.padding_x(CONTROL_INSET)
}

pub(crate) fn control_surface(
    node: NodeBuilder,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    overlay: StyleOverlay,
) -> NodeBuilder {
    state_surface(node.class(CONTROL_BASE), interaction, disabled, overlay)
}

pub(crate) fn state_surface(
    node: NodeBuilder,
    interaction: InteractionState,
    disabled: Option<Signal<bool>>,
    overlay: StyleOverlay,
) -> NodeBuilder {
    disabled_a11y(node, disabled)
        .stateful_style(CONTROL_VARIANT, overlay, move || {
            reactive_style_state(interaction, disabled)
        })
        .seal_style()
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

fn toggle_bool(value: Signal<bool>, on_change: &BoolChange) {
    let next = !value.peek();
    value.set(next);
    on_change(next);
}
