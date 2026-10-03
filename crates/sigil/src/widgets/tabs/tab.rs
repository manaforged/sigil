use std::cell::Cell;
use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::{Key, NodeEventsExt};
use crate::kernel::{AccessibleName, IntoNode, NodeBuilder, Role, TextBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::{
    ClassSource, ColorAtom, DISABLED_OPACITY, Px, Radius, Rgba8, StateSlot, StateStyle, StyleAtom,
    StyleOverlay, StyleVariant, Token, space,
};
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{Activate, activate_key, activate_pointer};

use super::builder::{CloseHandler, STRIP_HEIGHT, StripChrome, TAB_HEIGHT};
use super::drag::{DragState, install_label_drag_hooks};
use crate::layout::snap::CellForm;

const TAB_PADDING_X: f32 = space(3);
const CLOSE_AFFORDANCE_WIDTH: f32 = space(4);
const LINE_STRIP: CellForm<f32> = CellForm::new(STRIP_HEIGHT, space(4));
const CLOSE_GAP: CellForm<f32> = CellForm::new(space(1), 0.0);

pub(super) fn line_strip_height() -> f32 {
    LINE_STRIP.pick()
}

static TAB_BASE: &[StyleAtom] = &[StyleAtom::Transition(presets::SMOOTH)];
static TAB_REST: &[StyleAtom] = &[Token::Surface.bg()];
static TAB_FOCUS: &[StyleAtom] = &[Token::SurfaceHover.bg()];
static TAB_HOVER: &[StyleAtom] = &[Token::SurfaceHover.bg()];
static TAB_DOWN: &[StyleAtom] = &[Token::SurfacePressed.bg()];
static TAB_DISABLED: &[StyleAtom] = &[StyleAtom::Opacity(DISABLED_OPACITY)];

static TAB_ACTIVE_REST: &[StyleAtom] = &[Token::SurfacePressed.bg()];
static TAB_ACTIVE_FOCUS: &[StyleAtom] = &[Token::SurfacePressedStrong.bg()];
static TAB_ACTIVE_HOVER: &[StyleAtom] = &[Token::SurfacePressedStrong.bg()];
static TAB_ACTIVE_DOWN: &[StyleAtom] = &[Token::SurfacePressedStrong.bg()];

static TAB_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(TAB_REST, TAB_FOCUS, TAB_HOVER, TAB_DOWN, TAB_DISABLED),
);

static TAB_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(
        TAB_ACTIVE_REST,
        TAB_ACTIVE_FOCUS,
        TAB_ACTIVE_HOVER,
        TAB_ACTIVE_DOWN,
        TAB_DISABLED,
    ),
);

static PILL_ACTIVE_REST: &[StyleAtom] = &[Token::Surface.bg()];
static PILL_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(&[], TAB_FOCUS, TAB_HOVER, TAB_DOWN, TAB_DISABLED),
);
static PILL_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(
        PILL_ACTIVE_REST,
        PILL_ACTIVE_REST,
        PILL_ACTIVE_REST,
        PILL_ACTIVE_REST,
        TAB_DISABLED,
    ),
);

static LINE_BASE: &[StyleAtom] = &[
    StyleAtom::Transition(presets::SMOOTH),
    StyleAtom::BorderBottom(Px(2)),
    StyleAtom::BorderColor(ColorAtom::Rgba8(Rgba8 {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    })),
];
static LINE_DISABLED: &[StyleAtom] = &[StyleAtom::Opacity(DISABLED_OPACITY)];
static LINE_ACTIVE: &[StyleAtom] = &[Token::Accent.border()];
static LINE_ACTIVE_HOVER: &[StyleAtom] = &[Token::Accent.border(), Token::SurfaceHover.bg()];
static LABEL_MUTED: &[StyleAtom] = &[Token::Gray11.text(), StyleAtom::FontWeight(500)];
static LABEL_ACTIVE: &[StyleAtom] = &[Token::OnSurface.text(), StyleAtom::FontWeight(600)];
static LINE_VARIANT: StyleVariant = StyleVariant::new(
    LINE_BASE,
    StateStyle::new(&[], TAB_FOCUS, TAB_HOVER, TAB_DOWN, LINE_DISABLED),
);
static LINE_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    LINE_BASE,
    StateStyle::new(
        LINE_ACTIVE,
        LINE_ACTIVE_HOVER,
        LINE_ACTIVE_HOVER,
        LINE_ACTIVE_HOVER,
        LINE_DISABLED,
    ),
);

fn tab_layout(chrome: StripChrome) -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(if matches!(chrome, StripChrome::Line) {
            line_strip_height()
        } else {
            TAB_HEIGHT
        })
        .padding_x(TAB_PADDING_X)
        .gap(CLOSE_GAP.pick())
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn tab_label(label: String, selected: Signal<bool>) -> TextBuilder {
    CellForm::<fn(String, Signal<bool>) -> TextBuilder>::new(window_label, cell_label).pick()(
        label, selected,
    )
}

fn cell_label(label: String, selected: Signal<bool>) -> TextBuilder {
    let marked = derived(move || {
        if selected.get() {
            format!("[{label}]")
        } else {
            format!(" {label} ")
        }
    });
    label_style(text(marked), selected)
}

fn window_label(label: String, selected: Signal<bool>) -> TextBuilder {
    label_style(text(label), selected)
}

fn label_style(label: TextBuilder, selected: Signal<bool>) -> TextBuilder {
    label
        .layout(LayoutStyle::block())
        .style_reactive(move || {
            if selected.get() {
                LABEL_ACTIVE
            } else {
                LABEL_MUTED
            }
        })
        .hittable(false)
}

fn close_button(index: usize, handler: Option<CloseHandler>) -> impl IntoNode {
    let interaction = InteractionState::new(true);
    let activate: Activate = Rc::new(move || {
        if let Some(cb) = handler.as_ref() {
            cb(index);
        }
    });
    let pointer_activate = Rc::clone(&activate);
    let node = interaction.install(
        container()
            .role(Role::Button)
            .label(AccessibleName::Static(String::from("Close tab")))
            .layout(LayoutStyle::block().fixed_size(CLOSE_AFFORDANCE_WIDTH, CLOSE_AFFORDANCE_WIDTH))
            .class(Radius::Sm.atom())
            .child(close_glyph()),
    );
    node.pointer_down(move |event| {
        event.stop_propagation();
    })
    .pointer_up(move |event| {
        event.stop_propagation();
        activate_pointer(event, interaction, &pointer_activate);
    })
}

fn close_glyph() -> impl IntoNode {
    text("\u{00D7}")
        .layout(LayoutStyle::block().width(CLOSE_AFFORDANCE_WIDTH))
        .class(Token::OnSurface.text())
}

fn tab_frame(
    chrome: StripChrome,
    interaction: InteractionState,
    selected: Signal<bool>,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let slot = Rc::new(Cell::new(StateSlot::Rest));
    let style_slot = Rc::clone(&slot);
    container()
        .layout(tab_layout(chrome))
        .class(if matches!(chrome, StripChrome::Line) {
            ClassSource::from("text-sm font-medium")
        } else {
            ClassSource::from(Radius::Md.atom())
        })
        .class(if matches!(chrome, StripChrome::Line) {
            LINE_BASE
        } else {
            TAB_BASE
        })
        .style_reactive(move || {
            let state = interaction.style_state();
            style_slot.set(state.slot());
            let variant = match (chrome, selected.get()) {
                (StripChrome::Line, true) => LINE_ACTIVE_VARIANT,
                (StripChrome::Line, false) => LINE_VARIANT,
                (StripChrome::Pill, true) => PILL_ACTIVE_VARIANT,
                (StripChrome::Pill, false) => PILL_VARIANT,
                (_, true) => TAB_ACTIVE_VARIANT,
                (_, false) => TAB_VARIANT,
            };
            variant.state(state)
        })
        .state_overlay(user_overlay, slot)
}

#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn build_tab(
    index: usize,
    visual_index: usize,
    label: String,
    active: Signal<usize>,
    disabled: &Rc<Vec<usize>>,
    drag: Option<DragState>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    user_overlay: StyleOverlay,
    chrome: StripChrome,
) -> NodeBuilder {
    let is_disabled = disabled.contains(&index);
    let interaction = InteractionState::new(!is_disabled);
    let selected = derived(move || active.get() == index);

    let drag_for_activate = drag;
    let activate: Activate = Rc::new(move || {
        if is_disabled {
            return;
        }
        if drag_for_activate.is_some_and(DragState::consume_just_dragged) {
            return;
        }
        active.set(index);
    });
    let pointer_activate = Rc::clone(&activate);
    let key_activate = activate;

    let mut node = tab_frame(chrome, interaction, selected, user_overlay)
        .role(Role::Tab)
        .label(label.clone())
        .selected(selected)
        .pointer_up(move |event| activate_pointer(event, interaction, &pointer_activate))
        .key(Key::Enter, move |event| {
            activate_key(event, interaction, &key_activate);
        });

    if let Some(drag_state) = drag {
        node = install_label_drag_hooks(node, drag_state, visual_index);
    }

    node = node.child(tab_label(label, selected));

    if closeable {
        node = node.child(close_button(index, close_handler.as_ref().map(Rc::clone)));
    }

    interaction.install(node)
}
