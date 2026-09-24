use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::prelude::{Key, LogicalKey, NodeEventsExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::prelude::{
    AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, cols, rows,
};
use crate::layout::snap::CellForm;
use crate::reactive::{Signal, derived};
use crate::style::{
    BORDER_WIDTH, ClassSource, Metrics, NodeStyleExt, Px, Size, StateHost, StateStyle, Style,
    StyleAccess, StyleAtom, StyleOverlay, StyleVariant, Theme, Token, Tone, TransitionId,
};

use crate::widgets::interaction::InteractionState;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::shared::{
    BoolChange, BuilderMutator, activate_binary_key, activate_binary_pointer, binary_style_state,
    control_row, control_surface, disabled_now, label, noop_bool_change,
};

const CHECKBOX_TRANSITION: TransitionId = presets::SMOOTH;
const CHECKBOX_BOX_SIZE: f32 = 18.0;
const CHECKBOX_MARK_SIZE: f32 = 12.0;

pub struct CheckboxBuilder {
    checked: Signal<bool>,
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
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
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
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
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
        node.layout(layout_overrides.apply(checkbox_layout(has_label)))
    }

    fn base_node(
        &self,
        interaction: InteractionState,
        on_change: BoolChange,
        user_overlay: StyleOverlay,
    ) -> NodeBuilder {
        let checked = self.checked;
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
            .child(checkbox_indicator(checked, disabled, tone, interaction));
        if let Some(label_source) = self.label.clone() {
            node = node.child(checkbox_label(label_source));
        }
        interaction.install(node)
    }
}

impl StyleAccess for CheckboxBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
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

fn checkbox_indicator(
    checked: Signal<bool>,
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
            form(tone, checked.get()).state(binary_style_state(interaction, disabled))
        })
        .child(CellForm::<fn(Signal<bool>) -> NodeBuilder>::new(
            checkbox_mark,
            cell_mark,
        )
        .pick()(checked))
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

fn cell_mark(checked: Signal<bool>) -> NodeBuilder {
    container().child(text(derived(move || {
        if checked.get() { "[x]" } else { "[ ]" }.to_owned()
    })))
}

fn checkbox_mark(checked: Signal<bool>) -> NodeBuilder {
    container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(CHECKBOX_MARK_SIZE, CHECKBOX_MARK_SIZE))
        .style_reactive(move || {
            if checked.get() {
                MARK_VISIBLE
            } else {
                MARK_HIDDEN
            }
        })
        .child(check_path())
}

fn check_path() -> super::vector::PathNode {
    super::vector::path(24, 24)
        .move_to(5.0, 12.5)
        .line_to(10.0, 18.0)
        .line_to(19.0, 6.5)
        .stroke(
            crate::style::Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            3.0,
        )
        .layout(LayoutStyle::block().fixed_size(CHECKBOX_MARK_SIZE, CHECKBOX_MARK_SIZE))
}

fn checkbox_label(source: TextContent) -> impl IntoNode {
    label(source).layout(LayoutStyle::block().fill_width())
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
