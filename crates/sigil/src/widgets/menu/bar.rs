use std::rc::Rc;

use crate::events::NodeEventsExt;
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container, text};
use crate::layout::PopupSide;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Owner, OwnerId, Signal, signal};
use crate::style::{
    BORDER_WIDTH, Metrics, Paint, PaintTarget, Radius, Size, StateVariants, Style, StyleAtom,
    StyleSlot, Token, space,
};

use super::super::interactive::interactive;
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
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

struct Slot {
    label: String,
    rows: Vec<Row>,
}

pub struct MenuBarBuilder {
    slots: Vec<Slot>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn menu_bar() -> MenuBarBuilder {
    MenuBarBuilder {
        slots: Vec::new(),
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl MenuBarBuilder {
    #[must_use]
    pub fn menu(
        mut self,
        label: impl Into<String>,
        items: impl IntoIterator<Item = MenuItem>,
    ) -> Self {
        self.slots.push(Slot {
            label: label.into(),
            rows: items.into_iter().map(Row::Item).collect(),
        });
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let open_idx: Signal<Option<usize>> = signal(None);
        let host = Owner::current();
        let mut bar = container()
            .role(Role::GenericContainer)
            .label("Menu bar")
            .with(self.layout_overrides.apply(bar_layout()))
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

impl WidgetLayoutExt for MenuBarBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl PaintTarget for MenuBarBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl IntoNode for MenuBarBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn bar_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .height(TRIGGER_H)
        .shrink(0.0)
        .padding_x(space(1))
        .gap(space(1))
        .align_items(AlignItems::Center)
}

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
    slot: Slot,
    open_idx: Signal<Option<usize>>,
    host: Option<OwnerId>,
) -> NodeBuilder {
    let trigger = interactive_trigger(slot.label.clone(), index, open_idx);
    let rows = slot.rows;
    let height = panel_height(&rows);
    let width = trigger_width(&slot.label);
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
                vec![slot_popup(&rows, height, open_idx, host)]
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
                .height(Metrics::current().control_height(Size::Compact))
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

fn slot_popup(
    rows: &[Row],
    height: f32,
    open_idx: Signal<Option<usize>>,
    host: Option<OwnerId>,
) -> NodeBuilder {
    let close: Rc<dyn Fn()> = Rc::new(move || open_idx.set(None));
    menu_panel(rows.to_vec(), close, PopupSide::Below, host).layout(panel_layout(
        TRIGGER_H + POPUP_GAP,
        0.0,
        PANEL_WIDTH,
        height,
    ))
}

#[cfg(test)]
#[path = "bar_tests.rs"]
mod tests;
