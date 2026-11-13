use crate::layout::Space;
use std::rc::Rc;

mod press;

use crate::clipboard::Clipboard;
use crate::kernel::{AccessibleName, NodeBuilder, TextContent};
use crate::layout::{LayoutStyle, Length, StyleData};
use crate::reactive::{Reactive, Signal, memo};
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
pub(in crate::widgets) type TextChange = Rc<dyn Fn(&str) + 'static>;

pub(crate) fn noop_activate() -> Activate {
    Rc::new(|| {})
}

pub(in crate::widgets) fn noop_change() -> TextChange {
    Rc::new(|_| {})
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
        (Some(left), Some(right)) => Some(memo(move || left.get() || right.get())),
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
    AccessibleName::from(label)
}

pub(crate) struct TextInputProps {
    pub(crate) value: Signal<String>,
    pub(crate) placeholder: TextContent,
    pub(crate) disabled: Option<Signal<bool>>,
    pub(crate) change: Option<TextChange>,
    pub(crate) submit: Option<TextChange>,
    pub(crate) clipboard: Option<Rc<dyn Clipboard>>,
    pub(crate) caret_visible: Option<Signal<bool>>,
}

impl core::fmt::Debug for TextInputProps {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TextInputProps")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("caret_visible", &self.caret_visible)
            .finish_non_exhaustive()
    }
}

impl TextInputProps {
    pub(crate) fn new(value: Signal<String>) -> Self {
        Self {
            value,
            placeholder: TextContent::Static(String::new()),
            disabled: None,
            change: None,
            submit: None,
            clipboard: None,
            caret_visible: None,
        }
    }

    pub(crate) fn placeholder(&mut self, text: impl Into<TextContent>) {
        self.placeholder = text.into();
    }

    pub(crate) fn disabled(&mut self, flag: impl Reactive<bool>) {
        self.disabled = Some(flag.into_signal());
    }

    pub(crate) fn add_change(&mut self, f: impl Fn(&str) + 'static) {
        self.change = Some(chain_text_change(self.change.take(), f));
    }

    pub(crate) fn add_submit(&mut self, f: impl Fn(&str) + 'static) {
        self.submit = Some(chain_text_change(self.submit.take(), f));
    }

    pub(crate) fn clipboard(&mut self, clipboard: impl Clipboard + 'static) {
        self.clipboard = Some(Rc::new(clipboard));
    }

    pub(crate) const fn caret_visible(&mut self, visible: Signal<bool>) {
        self.caret_visible = Some(visible);
    }
}

fn chain_text_change(before: Option<TextChange>, f: impl Fn(&str) + 'static) -> TextChange {
    match before {
        Some(before) => Rc::new(move |text| {
            before(text);
            f(text);
        }),
        None => Rc::new(f),
    }
}

pub(crate) fn caller_style(node: NodeBuilder, style: Style) -> NodeBuilder {
    if style.is_empty() {
        node
    } else {
        node.style(style)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewportSizing {
    Fixed,
    Fill,
    Fit,
}

impl ViewportSizing {
    pub(crate) const fn of(fit: bool, fills: bool) -> Self {
        if fit {
            Self::Fit
        } else if fills {
            Self::Fill
        } else {
            Self::Fixed
        }
    }

    pub(crate) fn measured(self, set: &StyleData) -> bool {
        let bounded = set.min_size.height != Length::Auto || set.max_size.height != Length::Auto;
        self != Self::Fixed || bounded
    }
}
