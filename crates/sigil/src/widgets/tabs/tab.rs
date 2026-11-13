use crate::layout::Space;
use std::cell::Cell;
use std::rc::Rc;

use crate::animation::runtime::presets;
use crate::kernel::{AccessibleName, IntoNode, NodeBuilder, Role, TextBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::reactive::{Signal, memo};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string_once;
use crate::style::{
    ClassSource, Color, ColorValue, DIMMED, Radius, StateSlot, StateStyle, StyleAtom, StyleOverlay,
    StyleVariant, Token, Tone, space,
};
use crate::widgets::Layout;
use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{Activate, RELEASE_ENTER, press};

use super::builder::{CloseHandler, STRIP_HEIGHT, StripChrome, TAB_HEIGHT};
use super::drag::{TabDrag, install_label_drag_hooks};
use crate::layout::snap::CellForm;

const CLOSE_AFFORDANCE_WIDTH: f32 = space(4);
const TAB_PADDING_X: f32 = space(3);
const LINE_STRIP: CellForm<f32> = CellForm::new(STRIP_HEIGHT, space(4));
const BARE_TAB: CellForm<f32> = CellForm::new(TAB_HEIGHT, crate::Host::CELL);
const CLOSE_GAP: CellForm<f32> = CellForm::new(space(1), 0.0);

pub(super) fn line_strip_height() -> f32 {
    LINE_STRIP.pick()
}

pub(super) fn tab_height(chrome: StripChrome) -> f32 {
    if matches!(chrome, StripChrome::Line) {
        line_strip_height()
    } else {
        BARE_TAB.pick()
    }
}

static TAB_BASE: &[StyleAtom] = &[StyleAtom::Transition(presets::SMOOTH)];
static TAB_REST: &[StyleAtom] = &[Token::Surface.background()];
static TAB_HOVER: &[StyleAtom] = &[Token::SurfaceHover.background()];
static TAB_DOWN: &[StyleAtom] = &[Token::SurfacePressed.background()];
static TAB_ACTIVE_LIT: &[StyleAtom] = &[Token::SurfacePressedStrong.background()];
static LINE_ACTIVE: &[StyleAtom] = &[Token::Accent.border()];
static LINE_ACTIVE_HOVER: &[StyleAtom] =
    &[Token::Accent.border(), Token::SurfaceHover.background()];
static LABEL_MUTED: &[StyleAtom] = &[Tone::Neutral.step(11).text(), StyleAtom::FontWeight(500)];
static LABEL_ACTIVE: &[StyleAtom] = &[Token::OnSurface.text(), StyleAtom::FontWeight(600)];
static LABEL_ACTIVE_CELL: &[StyleAtom] = &[Token::OnSurface.text(), StyleAtom::FontWeight(700)];
static LINE_BASE: &[StyleAtom] = &[
    StyleAtom::Transition(presets::SMOOTH),
    StyleAtom::BorderBottom(2),
    StyleAtom::BorderColor(ColorValue::Color(Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    })),
];

static TAB_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(TAB_REST, TAB_HOVER, TAB_HOVER, TAB_DOWN, DIMMED),
);
static TAB_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(
        TAB_DOWN,
        TAB_ACTIVE_LIT,
        TAB_ACTIVE_LIT,
        TAB_ACTIVE_LIT,
        DIMMED,
    ),
);
static PILL_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(&[], TAB_HOVER, TAB_HOVER, TAB_DOWN, DIMMED),
);
static PILL_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    TAB_BASE,
    StateStyle::new(TAB_REST, TAB_REST, TAB_REST, TAB_REST, DIMMED),
);
static LINE_VARIANT: StyleVariant = StyleVariant::new(
    LINE_BASE,
    StateStyle::new(&[], TAB_HOVER, TAB_HOVER, TAB_DOWN, DIMMED),
);
static LINE_ACTIVE_VARIANT: StyleVariant = StyleVariant::new(
    LINE_BASE,
    StateStyle::new(
        LINE_ACTIVE,
        LINE_ACTIVE_HOVER,
        LINE_ACTIVE_HOVER,
        LINE_ACTIVE_HOVER,
        DIMMED,
    ),
);

fn tab_label(label: String, selected: Signal<bool>) -> TextBuilder {
    let cell = CellForm::new(false, true).pick();
    let node = if cell {
        text(format!(" {label} "))
    } else {
        text(label)
    };
    let active = if cell {
        LABEL_ACTIVE_CELL
    } else {
        LABEL_ACTIVE
    };
    node.layout(LayoutStyle::block())
        .style_reactive(move || if selected.get() { active } else { LABEL_MUTED })
        .hittable(false)
}

fn close_button(index: usize, handler: Option<CloseHandler>) -> impl IntoNode {
    container()
        .role(Role::Button)
        .label(AccessibleName::Static(theme_string_once(
            StringKey::CloseTab,
        )))
        .layout(LayoutStyle::block().fixed_size(CLOSE_AFFORDANCE_WIDTH, CLOSE_AFFORDANCE_WIDTH))
        .class(Radius::Sm.atom())
        .child(
            text(crate::widgets::icon::DISMISS_GLYPH)
                .layout(LayoutStyle::block().width(CLOSE_AFFORDANCE_WIDTH))
                .class(Token::OnSurface.text()),
        )
        .activate(move || {
            if let Some(cb) = handler.as_ref() {
                cb(index);
            }
        })
}

fn tab_frame(
    chrome: StripChrome,
    interaction: InteractionState,
    selected: Signal<bool>,
    user_overlay: StyleOverlay,
) -> NodeBuilder {
    let slot = Rc::new(Cell::new(StateSlot::Rest));
    let style_slot = Rc::clone(&slot);
    let line = matches!(chrome, StripChrome::Line);
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(tab_height(chrome))
                .padding_x(Space::px(TAB_PADDING_X))
                .gap(Space::px(CLOSE_GAP.pick()))
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class(if line {
            ClassSource::from("text-sm font-medium")
        } else {
            ClassSource::from(Radius::Md.atom())
        })
        .class(if line { LINE_BASE } else { TAB_BASE })
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
    drag: Option<TabDrag>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    user_overlay: StyleOverlay,
    chrome: StripChrome,
) -> NodeBuilder {
    let is_disabled = disabled.contains(&index);
    let interaction = InteractionState::new(!is_disabled);
    let selected = memo(move || active.get() == index);

    let dragged = drag.clone();
    let activate: Activate = Rc::new(move || {
        if !is_disabled && !dragged.as_ref().is_some_and(TabDrag::consume_just_dragged) {
            active.set(index);
        }
    });
    let mut node = press(
        tab_frame(chrome, interaction, selected, user_overlay)
            .role(Role::Tab)
            .label(label.clone())
            .selected(selected),
        interaction,
        None,
        &activate,
        RELEASE_ENTER,
    );
    if let Some(drag_state) = drag {
        node = install_label_drag_hooks(node, drag_state, visual_index);
    }
    node = node.child(tab_label(label, selected));
    if closeable {
        node = node.child(close_button(index, close_handler));
    }
    interaction.install(node)
}
