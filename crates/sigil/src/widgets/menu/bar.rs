use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, container, text};
use crate::layout::PopupSide;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Owner, Signal, signal};
use crate::style::StateVariants;
use crate::style::{
    BORDER_WIDTH, ControlSize, Metrics, Paint, Radius, Style, StyleAtom, Token, space,
};
use crate::widgets::Widget;

use super::super::interactive::interactive;
use super::super::layout_ext::LayoutOverrides;
use super::super::popup::{PANEL_WIDTH, POPUP_GAP};
use super::builder::MenuItem;
use super::panel::{Row, menu_panel, panel_height, panel_layout};

const TRIGGER_H: f32 = space(8);

static BAR_SURFACE: &[StyleAtom] = &[
    Token::SurfaceSubtle.bg(),
    StyleAtom::BorderBottom(BORDER_WIDTH),
    Token::Border.border(),
];
static TRIGGER_REST: &[StyleAtom] = &[Radius::Md.atom()];
static TRIGGER_OPEN: &[StyleAtom] = &[Radius::Md.atom(), Token::SurfacePressed.bg()];

struct MenuSlot {
    label: String,
    rows: Vec<Row>,
}

/// Builder of a menu bar.
pub struct MenuBarBuilder {
    slots: Vec<MenuSlot>,
    fill: bool,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// An in-window menu bar, as wide as its menus.
#[must_use]
pub fn menu_bar() -> MenuBarBuilder {
    MenuBarBuilder {
        slots: Vec::new(),
        fill: false,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl MenuBarBuilder {
    /// Sets the menu.
    #[must_use]
    pub fn menu(
        mut self,
        label: impl Into<String>,
        items: impl IntoIterator<Item = MenuItem>,
    ) -> Self {
        self.slots.push(MenuSlot {
            label: label.into(),
            rows: items.into_iter().map(Row::Item).collect(),
        });
        self
    }

    /// Applies the fill option.
    #[must_use]
    pub const fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    fn root(self) -> NodeBuilder {
        let open_idx: Signal<Option<usize>> = signal(None);
        let host = Owner::current();
        let mut bar = container()
            .role(Role::GenericContainer)
            .label(theme_label(StringKey::MenuBar))
            .with({
                let row = LayoutStyle::flex_row();
                let row = if self.fill { row.fill_width() } else { row };
                row.height(TRIGGER_H)
                    .shrink(0.0)
                    .padding_x(space(1))
                    .gap(space(1))
                    .align_items(AlignItems::Center)
            })
            .class(BAR_SURFACE);

        if !self.style_accumulator.is_empty() {
            bar = bar.style(self.style_accumulator);
        }

        for (index, slot) in self.slots.into_iter().enumerate() {
            bar = bar.child(slot_node(index, slot, open_idx, host));
        }

        bar
    }
}

builder_impls!([] MenuBarBuilder: paint(style_accumulator), widget, node);

#[allow(
    clippy::cast_precision_loss,
    reason = "char counts are tiny; exactness is irrelevant at pixel scale"
)]
fn trigger_width(label: &str) -> f32 {
    const PAD: f32 = space(4);
    const EM: f32 = space(2);
    (label.chars().count() as f32).mul_add(EM, PAD)
}

fn slot_node(
    index: usize,
    slot: MenuSlot,
    open_idx: Signal<Option<usize>>,
    host: Option<Owner>,
) -> NodeBuilder {
    let width = trigger_width(&slot.label);
    let trigger = interactive_trigger(slot.label, index, open_idx);
    let rows = slot.rows;
    let height = panel_height(&rows);
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(TRIGGER_H)
                .width(width)
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .child(trigger)
        .children_reactive(move || {
            if open_idx.get() == Some(index) {
                let close: Rc<dyn Fn()> = Rc::new(move || open_idx.set(None));
                vec![
                    menu_panel(rows.clone(), close, PopupSide::Below, host).layout(panel_layout(
                        TRIGGER_H + POPUP_GAP,
                        0.0,
                        PANEL_WIDTH,
                        height,
                    )),
                ]
            } else {
                Vec::new()
            }
        })
}

fn interactive_trigger(
    label: String,
    index: usize,
    open_idx: Signal<Option<usize>>,
) -> impl IntoNode {
    let inner = container()
        .role(Role::Button)
        .label(label.clone())
        .hittable(true)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(Metrics::current().control_height(ControlSize::Compact))
                .padding_x(space(2))
                .align_items(AlignItems::Center),
        )
        .style_reactive(move || {
            if open_idx.get() == Some(index) {
                TRIGGER_OPEN
            } else {
                TRIGGER_REST
            }
        })
        .pointer_up(move |event| {
            event.stop_propagation();
            if open_idx.peek() == Some(index) {
                open_idx.set(None);
            } else {
                open_idx.set(Some(index));
            }
        })
        .child(text(label).class("text-sm").class(Token::OnSurface.text()));
    interactive(inner)
        .hover(|s| s.bg(Token::SurfaceHover))
        .pressed(|s| s.bg(Token::SurfacePressed))
}

#[cfg(test)]
#[path = "bar_tests.rs"]
mod tests;
