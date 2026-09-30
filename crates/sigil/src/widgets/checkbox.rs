use crate::reactive::Reactive;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::{Key, LogicalKey, NodeEventsExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::{Signal, derived, signal};
use crate::style::{
    BORDER_WIDTH, Metrics, NodeStyleExt, PaintTarget, Px, Size, StateHost, StateStyle, Style,
    StyleAtom, StyleOverlay, StyleSlot, StyleVariant, Theme, Token, Tone, TransitionId,
};

use crate::widgets::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    BoolChange, BuilderMutator, activate_binary_key, activate_binary_pointer, binary_style_state,
    control_row, control_surface, disabled_now, noop_bool_change,
};

const CHECKBOX_TRANSITION: TransitionId = presets::SMOOTH;
const CHECKBOX_BOX_SIZE: f32 = 16.0;
const CHECKBOX_MARK_SIZE: f32 = 12.0;

pub struct CheckboxBuilder {
    checked: Signal<bool>,
    partial: Option<Signal<bool>>,
    label: Option<TextContent>,
    disabled: Option<Signal<bool>>,
    tone: Tone,
    on_change: Option<BoolChange>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

static EMPTY_BASE: &[StyleAtom] = &[];
static PRIMARY_CHECKED: &[StyleAtom] = &[Token::Accent.bg(), Token::Accent.border()];
static PRIMARY_CHECKED_HOVER: &[StyleAtom] =
    &[Token::AccentHover.bg(), Token::AccentHover.border()];
static PRIMARY_CHECKED_DOWN: &[StyleAtom] = &[Token::Accent.bg(), Token::Accent.border()];
static NEUTRAL_CHECKED: &[StyleAtom] = &[Token::Gray9.bg(), Token::Gray9.border()];
static NEUTRAL_CHECKED_HOVER: &[StyleAtom] = &[Token::Gray10.bg(), Token::Gray10.border()];
static NEUTRAL_CHECKED_DOWN: &[StyleAtom] = &[Token::Gray10.bg(), Token::Gray10.border()];
static UNCHECKED: &[StyleAtom] = &[Token::Field.bg()];
static UNCHECKED_HOVER: &[StyleAtom] = &[Token::SurfacePressed.bg()];
static UNCHECKED_DOWN: &[StyleAtom] = &[Token::SurfacePressedStrong.bg()];
static CHECKBOX_DISABLED: &[StyleAtom] = &[Token::SurfaceHover.bg()];
static BOX_BASE: &[StyleAtom] = &[
    StyleAtom::BorderRadius(Px(4)),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Border.border(),
    StyleAtom::Transition(CHECKBOX_TRANSITION),
];
static CELL_FOCUS: &[StyleAtom] = &[Token::Accent.bg(), Token::OnAccent.text()];
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

#[must_use]
pub fn checkbox(checked: Signal<bool>) -> CheckboxBuilder {
    CheckboxBuilder::new(checked)
}

impl CheckboxBuilder {
    #[must_use]
    pub fn new(checked: Signal<bool>) -> Self {
        Self {
            checked,
            partial: None,
            label: None,
            disabled: None,
            tone: Tone::Primary,
            on_change: None,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub const fn indeterminate(mut self, partial: Signal<bool>) -> Self {
        self.partial = Some(partial);
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub fn change(mut self, change: impl Fn(bool) + 'static) -> Self {
        self.on_change = Some(Rc::new(change));
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(noop_bool_change, Rc::clone);
        let layout_overrides = self.layout_overrides.clone();
        let has_label = self.label.is_some();
        let user_overlay = self.style_accumulator.take_state_overlay();
        let mut node = self.base_node(interaction, Rc::clone(&on_change), user_overlay);
        for mutate in self.mutators {
            node = mutate(node);
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node.with(layout_overrides.apply(checkbox_layout(has_label)))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        on_change: BoolChange,
        user_overlay: StyleOverlay,
    ) -> NodeBuilder {
        let checked = self.checked;
        let partial = self.partial.unwrap_or_else(|| signal(false));
        let disabled = self.disabled;
        let tone = self.tone;
        let pointer_change = Rc::clone(&on_change);
        let enter_change = Rc::clone(&on_change);
        let space_change = on_change;
        let root = container()
            .role(Role::CheckBox)
            .label(accessible_label(self.label.as_ref()))
            .checked(checked)
            .layout(checkbox_layout(self.label.is_some()));
        let mut node = control_surface(root, interaction, disabled, user_overlay)
            .pointer_up(move |event| {
                activate_binary_pointer(event, interaction, disabled, checked, &pointer_change);
            })
            .key(Key::Enter, move |event| {
                activate_binary_key(event, interaction, disabled, checked, &enter_change);
            })
            .key(LogicalKey::character(" "), move |event| {
                activate_binary_key(event, interaction, disabled, checked, &space_change);
            })
            .child(checkbox_indicator(
                CheckMark { checked, partial },
                disabled,
                tone,
                interaction,
            ));
        if let Some(label_source) = self.label.clone() {
            node = node.child(checkbox_label(label_source));
        }
        interaction.install(node)
    }
}

impl PaintTarget for CheckboxBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl StateHost for CheckboxBuilder {}

impl WidgetLayoutExt for CheckboxBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for CheckboxBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn checkbox_layout(has_label: bool) -> LayoutStyle {
    let metrics = Metrics::current();
    let root_size = metrics.control_height(Size::Default);
    let gap = metrics.gap(Size::Default);
    let row = control_row(LayoutStyle::flex_row())
        .height(root_size)
        .gap(gap)
        .align_items(AlignItems::Center);
    if has_label {
        row.fill_width()
    } else {
        row.width(root_size)
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
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn cell_layout() -> LayoutStyle {
    LayoutStyle::block().width(cols(4.0)).height(rows(1.0))
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

fn checkbox_label(source: TextContent) -> impl IntoNode {
    crate::kernel::text(source)
        .nowrap()
        .layout(LayoutStyle::block().fill_width())
}

fn accessible_label(label: Option<&TextContent>) -> String {
    label
        .and_then(TextContent::as_static_str)
        .map_or_else(|| String::from("Checkbox"), ToOwned::to_owned)
}

const fn checkbox_variant(tone: Tone, checked: bool) -> StyleVariant {
    match (tone, checked) {
        (Tone::Neutral, true) => StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                NEUTRAL_CHECKED,
                NEUTRAL_CHECKED_HOVER,
                NEUTRAL_CHECKED_HOVER,
                NEUTRAL_CHECKED_DOWN,
                NEUTRAL_CHECKED,
            ),
        ),
        (_, true) => StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                PRIMARY_CHECKED,
                PRIMARY_CHECKED_HOVER,
                PRIMARY_CHECKED_HOVER,
                PRIMARY_CHECKED_DOWN,
                PRIMARY_CHECKED,
            ),
        ),
        (_, false) => StyleVariant::new(
            EMPTY_BASE,
            StateStyle::new(
                UNCHECKED,
                UNCHECKED_HOVER,
                UNCHECKED_HOVER,
                UNCHECKED_DOWN,
                CHECKBOX_DISABLED,
            ),
        ),
    }
}
