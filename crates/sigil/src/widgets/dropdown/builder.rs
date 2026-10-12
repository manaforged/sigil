use crate::reactive::Reactive;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::NodeEventsExt;
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Overflow, rows};
use crate::reactive::{Signal, derived, signal};
use crate::style::{
    BORDER_WIDTH, PaintTarget, Radius, StateHost, StateStyle, Style, StyleAtom, StyleOverlay,
    StyleSlot, StyleVariant, Token, TransitionId, space,
};

use crate::widgets::interaction::InteractionState;

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

use super::super::listbox::listbox;
use super::super::popup::{ChevronDirection, chevron};
use super::super::shared::{
    Activate, DIMMED, activate_pointer_reactive, disabled_now, reactive_style_state,
};
use super::popup::{DropdownModel, OptionCtx, SURFACE_HOVER, build_popup, trigger_height};

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;

pub(super) type IndexChange = Rc<dyn Fn(usize) + 'static>;

const DROPDOWN_TRANSITION: TransitionId = presets::SMOOTH;
const MIN_TRIGGER_WIDTH: f32 = space(24);
const LABEL_HEIGHT: f32 = 24.0;
static DROPDOWN_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(DROPDOWN_TRANSITION)];

static TRIGGER_BASE: &[StyleAtom] = &[
    Token::Field.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(7),
];
static TRIGGER_DOWN: &[StyleAtom] = &[Token::SurfacePressed.bg()];

static TRIGGER_VARIANT: StyleVariant = StyleVariant::new(
    TRIGGER_BASE,
    StateStyle::new(&[], SURFACE_HOVER, SURFACE_HOVER, TRIGGER_DOWN, DIMMED),
);

pub struct DropdownBuilder {
    selected: Signal<usize>,
    options: Vec<String>,
    label: String,
    disabled: Option<Signal<bool>>,
    on_change: Option<IndexChange>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn dropdown(selected: Signal<usize>, options: Vec<String>) -> DropdownBuilder {
    DropdownBuilder::new(selected, options)
}

impl DropdownBuilder {
    #[must_use]
    pub fn new(selected: Signal<usize>, options: Vec<String>) -> Self {
        Self {
            selected,
            options,
            label: "Dropdown".to_owned(),
            disabled: None,
            on_change: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    #[must_use]
    pub fn change(mut self, cb: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Rc::new(cb));
        self
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let enabled = !disabled_now(self.disabled);
        let interaction = InteractionState::new(enabled);
        let open = signal(false);
        let cursor = signal(self.selected.peek());
        let on_change = self
            .on_change
            .as_ref()
            .map_or_else(|| Rc::new(|_| {}), Rc::clone);
        let options = Rc::new(self.options);
        let ctx = OptionCtx {
            selected: self.selected,
            open,
            cursor,
            disabled: self.disabled,
            width: self.layout_overrides.width_hint(),
            trigger_height: self
                .layout_overrides
                .height_hint()
                .unwrap_or_else(trigger_height),
        };
        let user_overlay = self.style_accumulator.take_state_overlay();
        let trigger = build_trigger(
            ctx,
            &options,
            interaction,
            self.style_accumulator,
            user_overlay,
        );
        let (popup_options, popup_on_change) = (Rc::clone(&options), Rc::clone(&on_change));
        let root = dropdown_root(
            ctx,
            self.layout_overrides,
            Rc::clone(&options),
            self.label,
            on_change,
        )
        .child(trigger)
        .children_reactive(move || {
            if ctx.open.get() {
                vec![build_popup(ctx, &popup_options, &popup_on_change)]
            } else {
                Vec::new()
            }
        });
        interaction.install(root)
    }
}

impl PaintTarget for DropdownBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl StateHost for DropdownBuilder {}

impl WidgetLayoutExt for DropdownBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for DropdownBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

fn dropdown_root(
    ctx: OptionCtx,
    overrides: LayoutOverrides,
    options: Rc<Vec<String>>,
    label: String,
    on_change: IndexChange,
) -> NodeBuilder {
    let root_layout = overrides.apply(
        LayoutStyle::block()
            .fill_width()
            .height(ctx.trigger_height)
            .min_size(MIN_TRIGGER_WIDTH, 0.0),
    );
    listbox(
        container()
            .role(Role::ComboBox)
            .expanded(ctx.open)
            .label(label)
            .layout(root_layout),
        DropdownModel {
            ctx,
            options,
            on_change,
        },
    )
}

fn build_trigger(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    interaction: InteractionState,
    styled: Style,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let (selected, open, cursor, disabled) = (ctx.selected, ctx.open, ctx.cursor, ctx.disabled);
    let mut node = container()
        .layout(
            LayoutStyle::flex_row()
                .fill()
                .padding_x(10.0)
                .gap(space(2))
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .class(Radius::Md.atom())
        .class(TRIGGER_VARIANT.base())
        .class(DROPDOWN_ANIM_BASE)
        .stateful_style(TRIGGER_VARIANT, user_overlay, move || {
            reactive_style_state(interaction, disabled)
        })
        .seal_style()
        .pointer_up(move |ev| {
            let cb: Activate = Rc::new(move || {
                open.set(!open.peek());
                cursor.set(selected.peek());
            });
            activate_pointer_reactive(ev, interaction, disabled, &cb);
        })
        .child(trigger_label(selected, Rc::clone(options)))
        .child(chevron(ChevronDirection::Down, 16.0));
    if !styled.is_empty() {
        node = node.style(styled);
    }
    node
}

fn trigger_label(selected: Signal<usize>, options: Rc<Vec<String>>) -> impl IntoNode {
    let label_sig = derived(move || options.get(selected.get()).cloned().unwrap_or_default());
    container()
        .layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .width(0.0)
                .height(CellForm::new(LABEL_HEIGHT, rows(1.0).px()).pick())
                .align_items(AlignItems::Center)
                .overflow_x(Overflow::Hidden),
        )
        .child(
            text(TextContent::Reactive(label_sig))
                .ellipsize()
                .layout(LayoutStyle::block().fill_width())
                .class(Token::OnSurface.text()),
        )
}
