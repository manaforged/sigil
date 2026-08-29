use std::rc::Rc;

use sigil_animation_runtime::presets;
use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Overflow};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{
    NodeStyleExt, Px, StateHost, StateStyle, Style, StyleAccess, StyleAtom, StyleOverlay,
    StyleVariant, Tone, TransitionId,
};

use crate::interaction::InteractionState;

use super::super::chart::tone_solid;
use super::super::shared::{
    Activate, BuilderMutator, activate_pointer_reactive, disabled_now, reactive_style_state,
};
use super::super::transition_seam::install_transition_animators;
use super::super::vector::path;
use super::popup::{OptionCtx, SURFACE_HOVER, TRIGGER_HEIGHT, build_popup};

pub(super) type IndexChange = Rc<dyn Fn(usize) + 'static>;

const DROPDOWN_TRANSITION: TransitionId = presets::SMOOTH;
const DROPDOWN_COMPONENT_ID: u64 = 0xD012_D000;
static DROPDOWN_ANIM_BASE: &[StyleAtom] = &[StyleAtom::Transition(DROPDOWN_TRANSITION)];

static TRIGGER_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("field"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderToneStep(7),
];
static TRIGGER_DOWN: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed")];
static TRIGGER_DISABLED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

static TRIGGER_VARIANT: StyleVariant = StyleVariant::new(
    TRIGGER_BASE,
    StateStyle::new(
        &[],
        SURFACE_HOVER,
        SURFACE_HOVER,
        TRIGGER_DOWN,
        TRIGGER_DISABLED,
    ),
);

pub struct DropdownBuilder {
    selected: Signal<usize>,
    options: Vec<String>,
    label: String,
    disabled: Option<Signal<bool>>,
    on_change: Option<IndexChange>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    width: Option<f32>,
    height: f32,
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
            mutators: Vec::new(),
            style_accumulator: Style::new(),
            width: None,
            height: TRIGGER_HEIGHT,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }

    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.height = px;
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

        let selected = self.selected;
        let options = Rc::new(self.options);
        let disabled = self.disabled;

        let ctx = OptionCtx {
            selected,
            open,
            cursor,
            disabled,
            width: self.width,
            trigger_height: self.height,
        };
        let user_overlay = self.style_accumulator.take_state_overlay();
        let trigger = build_trigger(
            ctx,
            &options,
            interaction,
            self.style_accumulator,
            user_overlay,
        );
        let popup_options = Rc::clone(&options);
        let popup_on_change = Rc::clone(&on_change);
        let mut root = dropdown_root(ctx, Rc::clone(&options), self.label)
            .child(trigger)
            .children_reactive(move || {
                if ctx.open.get() {
                    vec![build_popup(ctx, &popup_options, &popup_on_change)]
                } else {
                    Vec::new()
                }
            });

        for mutate in self.mutators {
            root = mutate(root);
        }

        interaction.install(root)
    }
}

impl StyleAccess for DropdownBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for DropdownBuilder {}

impl IntoNode for DropdownBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn dropdown_root(ctx: OptionCtx, options: Rc<Vec<String>>, label: String) -> NodeBuilder {
    let OptionCtx {
        open,
        cursor,
        disabled,
        width,
        trigger_height,
        ..
    } = ctx;
    let root_layout = width.map_or_else(
        || LayoutStyle::block().fill_width().height(trigger_height),
        |px| LayoutStyle::block().width(px).height(trigger_height),
    );
    container()
        .role(Role::ComboBox)
        .label(label)
        .layout(root_layout)
        .key(Key::Escape, move |ev| {
            ev.stop_propagation();
            open.set(false);
        })
        .key(Key::ArrowDown, move |ev| {
            ev.stop_propagation();
            if disabled_now(disabled) {
                return;
            }
            if !open.peek() {
                open.set(true);
                return;
            }
            let n = options.len();
            if n > 0 {
                cursor.set((cursor.peek() + 1).min(n - 1));
            }
        })
        .key(Key::ArrowUp, move |ev| {
            ev.stop_propagation();
            if disabled_now(disabled) {
                return;
            }
            if cursor.peek() > 0 {
                cursor.set(cursor.peek() - 1);
            }
        })
}

fn build_trigger(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    interaction: InteractionState,
    styled: Style,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let label_options = Rc::clone(options);
    let OptionCtx {
        selected,
        open,
        cursor,
        disabled,
        width: _,
        trigger_height: _,
    } = ctx;
    let state_style = dropdown_trigger_state_style();
    let mut node = container()
        .role(Role::Button)
        .label("Select option")
        .layout(trigger_layout())
        .class("rounded-md")
        .class(TRIGGER_VARIANT.base())
        .class(DROPDOWN_ANIM_BASE)
        .stateful_style(TRIGGER_VARIANT, user_overlay, move || {
            reactive_style_state(interaction, disabled)
        })
        .pointer_up(move |ev| {
            let cb: Activate = Rc::new(move || {
                open.set(!open.peek());
                cursor.set(selected.peek());
            });
            activate_pointer_reactive(ev, interaction, disabled, &cb);
        })
        .key(Key::Enter, move |ev| {
            if interaction.activate_key(ev) && !disabled_now(disabled) {
                open.set(!open.peek());
                cursor.set(selected.peek());
            }
        })
        .child(trigger_label(selected, label_options))
        .child(trigger_chevron());
    node = install_transition_animators(
        node,
        DROPDOWN_ANIM_BASE,
        state_style,
        DROPDOWN_COMPONENT_ID,
        interaction,
    );
    if !styled.is_empty() {
        node = node.style(styled);
    }
    node
}

fn trigger_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill()
        .padding_x(10.0)
        .gap(8.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
}

const fn dropdown_trigger_state_style() -> StateStyle {
    StateStyle::new(
        &[],
        SURFACE_HOVER,
        SURFACE_HOVER,
        TRIGGER_DOWN,
        TRIGGER_DISABLED,
    )
}

fn trigger_label(selected: Signal<usize>, options: Rc<Vec<String>>) -> impl IntoNode {
    let label_sig = derived_eq(move || options.get(selected.get()).cloned().unwrap_or_default());
    container()
        .layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .width(0.0)
                .height(24.0)
                .align_items(AlignItems::Center)
                .overflow_x(Overflow::Hidden),
        )
        .child(
            text(TextContent::Reactive(label_sig))
                .layout(LayoutStyle::block().fill_width())
                .class("text-on-surface"),
        )
}

fn trigger_chevron() -> impl IntoNode {
    path(16, 16)
        .move_to(3.5, 5.4)
        .line_to(4.6, 4.3)
        .line_to(8.0, 7.7)
        .line_to(11.4, 4.3)
        .line_to(12.5, 5.4)
        .line_to(8.0, 9.9)
        .close()
        .fill(tone_solid(Tone::Neutral))
        .fit_size(16.0)
}
