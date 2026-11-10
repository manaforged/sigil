use crate::layout::Space;
use crate::reactive::Reactive;
use crate::style::strings::StringKey;
use std::rc::Rc;

use crate::animation::runtime::presets;
use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, cols, rows};
use crate::reactive::{Signal, derived, signal};
use crate::style::{
    BORDER_WIDTH, ControlSize, Metrics, StateStyle, Style, StyleAtom, StyleVariant, Token, Tone,
};
use crate::widgets::Layout;

use crate::widgets::interaction::InteractionState;

use super::layout_ext::LayoutOverrides;
use super::shared::{
    BoolChange, accessible_name_or, binary_style_state, caller_style, control, control_row, flip,
    noop_bool_change,
};

const CHECKBOX_BOX_SIZE: f32 = 16.0;
const CHECKBOX_MARK_SIZE: f32 = 12.0;

/// Builder of a checkbox.
pub struct CheckboxBuilder {
    checked: Signal<bool>,
    partial: Option<Signal<bool>>,
    label: Option<TextContent>,
    disabled: Option<Signal<bool>>,
    tone: Tone,
    on_change: Option<BoolChange>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for CheckboxBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CheckboxBuilder")
            .field("checked", &self.checked)
            .field("partial", &self.partial)
            .field("label", &self.label)
            .field("disabled", &self.disabled)
            .field("tone", &self.tone)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

static EMPTY_BASE: &[StyleAtom] = &[];
static PRIMARY_CHECKED: &[StyleAtom] = &[Token::Accent.background(), Token::Accent.border()];
static PRIMARY_CHECKED_HOVER: &[StyleAtom] =
    &[Token::AccentHover.background(), Token::AccentHover.border()];
static NEUTRAL_CHECKED: &[StyleAtom] = &[Token::Gray9.background(), Token::Gray9.border()];
static NEUTRAL_CHECKED_HOVER: &[StyleAtom] = &[Token::Gray10.background(), Token::Gray10.border()];
static UNCHECKED: &[StyleAtom] = &[Token::Field.background()];
static UNCHECKED_HOVER: &[StyleAtom] = &[Token::SurfacePressed.background()];
static UNCHECKED_DOWN: &[StyleAtom] = &[Token::SurfacePressedStrong.background()];
static CHECKBOX_DISABLED: &[StyleAtom] = &[Token::SurfaceHover.background()];
static BOX_BASE: &[StyleAtom] = &[
    StyleAtom::BorderRadius(4),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Border.border(),
    StyleAtom::Transition(presets::SMOOTH),
];
static CELL_FOCUS: &[StyleAtom] = &[Token::Accent.background(), Token::OnAccent.text()];
static CELL_DISABLED: &[StyleAtom] = &[Token::OnSurfaceMuted.text()];
static CELL_VARIANT: StyleVariant = StyleVariant::new(
    EMPTY_BASE,
    StateStyle::new(
        EMPTY_BASE,
        CELL_FOCUS,
        EMPTY_BASE,
        EMPTY_BASE,
        CELL_DISABLED,
    ),
);
static MARK_VISIBLE: &[StyleAtom] = &[StyleAtom::Opacity(10_000)];
static MARK_HIDDEN: &[StyleAtom] = &[StyleAtom::Opacity(0)];

/// A checkbox.
#[must_use]
pub fn checkbox(checked: Signal<bool>) -> CheckboxBuilder {
    CheckboxBuilder::new(checked)
}

impl CheckboxBuilder {
    /// Creates a checkbox builder.
    #[must_use]
    pub fn new(checked: Signal<bool>) -> Self {
        Self {
            checked,
            partial: None,
            label: None,
            disabled: None,
            tone: Tone::Primary,
            on_change: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Sets the indeterminate.
    #[must_use]
    pub const fn indeterminate(mut self, partial: Signal<bool>) -> Self {
        self.partial = Some(partial);
        self
    }

    /// Sets the tone.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the change.
    #[must_use]
    pub fn change(mut self, change: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(noop_bool_change, Rc::clone);
        let style = std::mem::take(&mut self.style_accumulator);
        let checked = self.checked;
        let partial = self.partial.unwrap_or_else(|| signal(false));
        let disabled = self.disabled;
        let mut node = control(
            container()
                .role(Role::CheckBox)
                .label(accessible_name_or(self.label.as_ref(), StringKey::Checkbox))
                .checked(checked)
                .mixed(partial)
                .layout(checkbox_layout(self.label.is_some())),
            disabled,
        );
        let interaction = node.interaction();
        node = node
            .activate_with(flip(checked, on_change))
            .child(checkbox_indicator(
                CheckMark { checked, partial },
                disabled,
                self.tone,
                interaction,
            ));
        if let Some(label) = self.label {
            node = node.child(
                text(label)
                    .nowrap()
                    .layout(LayoutStyle::block().fill_width()),
            );
        }
        caller_style(node, style)
    }
}

builder_impls!([] CheckboxBuilder: paint(style_accumulator), widget, states(style_accumulator), node);

fn checkbox_layout(has_label: bool) -> LayoutStyle {
    let metrics = Metrics::current();
    let root_size = metrics.control_height(ControlSize::Default);
    let gap = metrics.control_gap(ControlSize::Default);
    let row = control_row(LayoutStyle::flex_row())
        .height(root_size)
        .gap(Space::px(gap))
        .align_items(AlignItems::Center);
    if has_label {
        row.fill_width()
    } else {
        row.width(root_size).justify_content(JustifyContent::Center)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CheckMark {
    pub checked: Signal<bool>,
    pub partial: Signal<bool>,
}

impl CheckMark {
    fn filled(self) -> bool {
        self.checked.get() || self.partial.get()
    }
}

pub(crate) fn checkbox_indicator(
    mark: CheckMark,
    disabled: Option<Signal<bool>>,
    tone: Tone,
    interaction: InteractionState,
) -> NodeBuilder {
    let form =
        CellForm::<fn(Tone, bool) -> StyleVariant>::new(checkbox_variant, cell_variant).pick();
    container()
        .layout(CellForm::<fn() -> LayoutStyle>::new(
            box_layout,
            cell_layout,
        )
        .pick()())
        .class(CellForm::new(BOX_BASE, EMPTY_BASE).pick())
        .style_reactive(move || {
            form(tone, mark.filled()).state(binary_style_state(interaction, disabled))
        })
        .child(CellForm::<fn(CheckMark) -> NodeBuilder>::new(
            checkbox_mark,
            cell_mark,
        )
        .pick()(mark))
}

fn box_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(CHECKBOX_BOX_SIZE, CHECKBOX_BOX_SIZE)
        .shrink(0.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn cell_layout() -> LayoutStyle {
    LayoutStyle::block()
        .width(cols(4.0))
        .height(rows(1.0))
        .shrink(0.0)
}

const fn cell_variant(_tone: Tone, _checked: bool) -> StyleVariant {
    CELL_VARIANT
}

fn cell_mark(mark: CheckMark) -> NodeBuilder {
    container().child(text(derived(move || {
        match (mark.partial.get(), mark.checked.get()) {
            (true, _) => "[-]",
            (false, true) => "[x]",
            (false, false) => "[ ]",
        }
        .to_owned()
    })))
}

fn checkbox_mark(mark: CheckMark) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(CHECKBOX_MARK_SIZE, CHECKBOX_MARK_SIZE))
        .style_reactive(move || {
            if mark.filled() {
                MARK_VISIBLE
            } else {
                MARK_HIDDEN
            }
        })
        .children_reactive(move || vec![mark_path(mark.partial.get())])
}

fn mark_path(partial: bool) -> NodeBuilder {
    let path = super::vector::path(24, 24);
    let path = if partial {
        path.move_to(5.0, 12.0).line_to(19.0, 12.0)
    } else {
        path.move_to(5.0, 12.5)
            .line_to(10.0, 18.0)
            .line_to(19.0, 6.5)
    };
    container().hittable(false).child(
        path.stroke(Token::OnAccent, 3.0)
            .layout(LayoutStyle::block().fixed_size(CHECKBOX_MARK_SIZE, CHECKBOX_MARK_SIZE)),
    )
}

const fn checkbox_variant(tone: Tone, checked: bool) -> StyleVariant {
    let (rest, hover, down, disabled) = match (tone, checked) {
        (Tone::Neutral, true) => (
            NEUTRAL_CHECKED,
            NEUTRAL_CHECKED_HOVER,
            NEUTRAL_CHECKED_HOVER,
            NEUTRAL_CHECKED,
        ),
        (_, true) => (
            PRIMARY_CHECKED,
            PRIMARY_CHECKED_HOVER,
            PRIMARY_CHECKED,
            PRIMARY_CHECKED,
        ),
        (_, false) => (
            UNCHECKED,
            UNCHECKED_HOVER,
            UNCHECKED_DOWN,
            CHECKBOX_DISABLED,
        ),
    };
    StyleVariant::new(
        EMPTY_BASE,
        StateStyle::new(rest, hover, hover, down, disabled),
    )
}
