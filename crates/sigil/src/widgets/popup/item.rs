use crate::kernel::{IntoNode, NodeBuilder, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Overflow};
use crate::reactive::Signal;
use crate::style::{
    DisplayKind, Metrics, NodeStyleExt, Radius, Size, StyleAtom, Token, Tone, space,
};

use super::super::chart::tone_solid;
use super::super::vector::path;

const GLYPH: f32 = space(4);

static ROW_REST: &[StyleAtom] = &[Radius::Md.atom()];
static ROW_HIGHLIGHT: &[StyleAtom] = &[Radius::Md.atom(), Token::SurfaceHover.bg()];
static HIDDEN: &[StyleAtom] = &[StyleAtom::Display(DisplayKind::None)];

pub(crate) struct ItemSlots {
    pub(crate) label: String,
    pub(crate) shortcut: Option<String>,
    pub(crate) chevron: bool,
    pub(crate) checked: Option<Signal<bool>>,
    pub(crate) disabled: bool,
}

impl ItemSlots {
    #[must_use]
    pub(crate) const fn new(label: String) -> Self {
        Self {
            label,
            shortcut: None,
            chevron: false,
            checked: None,
            disabled: false,
        }
    }
}

#[must_use]
pub(crate) fn item_row(slots: ItemSlots, highlight: Signal<bool>) -> NodeBuilder {
    let disabled = slots.disabled;
    let mut row = container()
        .label(slots.label.clone())
        .layout(row_layout())
        .style_reactive(move || {
            if !disabled && highlight.get() {
                ROW_HIGHLIGHT
            } else {
                ROW_REST
            }
        })
        .child(label_text(slots.label, disabled));
    if let Some(hint) = slots.shortcut {
        row = row.child(
            text(hint)
                .class("text-sm")
                .class(Token::OnSurfaceMuted.text()),
        );
    }
    if let Some(checked) = slots.checked {
        row = row.child(check_mark(checked));
    }
    if slots.chevron {
        row = row.child(glyph_slot().child(chevron(ChevronDirection::Right, GLYPH)));
    }
    row
}

fn row_layout() -> LayoutStyle {
    let metrics = Metrics::current();
    LayoutStyle::flex_row()
        .fill_width()
        .height(metrics.control_height(Size::Default))
        .shrink(0.0)
        .padding_x(metrics.padding(Size::Default))
        .gap(metrics.gap(Size::Default))
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .overflow_x(Overflow::Hidden)
}

fn label_text(label: String, disabled: bool) -> impl IntoNode {
    let tone = if disabled {
        Token::OnSurfaceMuted
    } else {
        Token::OnSurface
    };
    text(label)
        .layout(LayoutStyle::block().grow(1.0))
        .class(tone.text())
}

fn glyph_slot() -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .fixed_size(GLYPH, GLYPH)
            .shrink(0.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    )
}

fn check_mark(checked: Signal<bool>) -> NodeBuilder {
    glyph_slot()
        .style_reactive(move || if checked.get() { &[] } else { HIDDEN })
        .child(check_glyph(GLYPH))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChevronDirection {
    Right,
    Down,
}

#[must_use]
fn check_glyph(size: f32) -> impl IntoNode {
    path(16, 16)
        .move_to(3.5, 8.5)
        .line_to(6.5, 11.5)
        .line_to(12.5, 4.5)
        .stroke(tone_solid(Tone::Neutral), 1.6)
        .fit_size(size)
}

#[must_use]
pub(crate) fn chevron(direction: ChevronDirection, size: f32) -> impl IntoNode {
    let shape = path(16, 16);
    let shape = match direction {
        ChevronDirection::Right => shape.move_to(6.0, 4.5).line_to(9.5, 8.0).line_to(6.0, 11.5),
        ChevronDirection::Down => shape.move_to(4.5, 6.0).line_to(8.0, 9.5).line_to(11.5, 6.0),
    };
    shape.stroke(tone_solid(Tone::Neutral), 1.6).fit_size(size)
}

#[must_use]
pub(crate) fn footer(actions: Vec<Box<dyn IntoNode>>) -> NodeBuilder {
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .shrink(0.0)
            .gap(space(2))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::End),
    );
    for action in actions {
        row = row.child(action);
    }
    row
}
