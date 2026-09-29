use std::rc::Rc;

use crate::events::NodeEventsExt;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::PopupSide;
use crate::layout::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, derived};
use crate::style::{Metrics, Size, StyleAtom, Token};

use super::super::listbox::{ListModel, scrolled};
use super::super::popup::{self, ItemSlots, POPUP_GAP, item_row};
use super::super::shared::disabled_now;
use super::builder::IndexChange;

pub(super) const TRIGGER_HEIGHT: f32 = 32.0;
pub(super) const POPUP_MAX_VISIBLE: usize = 6;

pub(super) static SURFACE_HOVER: &[StyleAtom] = &[Token::SurfaceHover.bg()];

#[derive(Clone, Copy)]
pub(super) struct OptionCtx {
    pub(super) selected: Signal<usize>,
    pub(super) open: Signal<bool>,
    pub(super) cursor: Signal<usize>,
    pub(super) disabled: Option<Signal<bool>>,
    pub(super) width: Option<f32>,
    pub(super) trigger_height: f32,
}

pub(super) struct DropdownModel {
    pub(super) ctx: OptionCtx,
    pub(super) options: Rc<Vec<String>>,
    pub(super) on_change: IndexChange,
}

impl ListModel for DropdownModel {
    fn cursor(&self) -> Signal<usize> {
        self.ctx.cursor
    }

    fn len(&self) -> usize {
        self.options.len()
    }

    fn label(&self, index: usize) -> Option<String> {
        self.options.get(index).cloned()
    }

    fn is_open(&self) -> bool {
        self.ctx.open.peek()
    }

    fn open(&self) {
        if disabled_now(self.ctx.disabled) {
            return;
        }
        self.ctx.cursor.set(self.ctx.selected.peek());
        self.ctx.open.set(true);
    }

    fn commit(&self, index: usize) {
        if disabled_now(self.ctx.disabled) {
            return;
        }
        self.ctx.selected.set(index);
        (self.on_change)(index);
        self.ctx.open.set(false);
    }

    fn close(&self) {
        self.ctx.open.set(false);
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "option counts are small; sub-unit precision is irrelevant at render scale"
)]
fn rows_height(count: usize) -> f32 {
    count as f32 * Metrics::current().control_height(Size::Default)
}

pub(super) fn build_popup(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    on_change: &IndexChange,
) -> NodeBuilder {
    let count = options.len();
    let view_h = rows_height(count.min(POPUP_MAX_VISIBLE));
    let extent = rows_height(count);
    let inner_w = ctx
        .width
        .map(|width| 2.0f32.mul_add(-popup::inset(), width).max(0.0));
    let base = LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_top(ctx.trigger_height + POPUP_GAP)
        .inset_left(0.0)
        .height(2.0f32.mul_add(popup::inset(), view_h))
        .padding(Metrics::current().overlay_padding());
    let frame = popup::popup(Role::ListBox, PopupSide::Below).layout(match ctx.width {
        Some(width) => base.width(width),
        None => base.fill_width(),
    });
    let mut list = container().layout(LayoutStyle::flex_column().fill_width().height(extent));
    for (index, label) in options.iter().enumerate() {
        list = list.child(build_option(
            index,
            label.clone(),
            ctx,
            Rc::clone(on_change),
        ));
    }
    if count > POPUP_MAX_VISIBLE {
        frame.child(scrolled(
            list,
            ctx.cursor,
            rows_height(1),
            extent,
            (inner_w, view_h),
        ))
    } else {
        frame.child(list)
    }
}

fn build_option(
    index: usize,
    label: String,
    ctx: OptionCtx,
    on_change: IndexChange,
) -> NodeBuilder {
    let checked = derived(move || ctx.selected.get() == index);
    let highlight = derived(move || ctx.cursor.get() == index);
    let slots = ItemSlots {
        checked: Some(checked),
        ..ItemSlots::new(label)
    };
    item_row(slots, highlight)
        .role(Role::ListBoxOption)
        .selected(checked)
        .pointer_up(move |event| {
            if event.is_captured() || disabled_now(ctx.disabled) {
                return;
            }
            ctx.selected.set(index);
            ctx.open.set(false);
            on_change(index);
            event.stop_propagation();
        })
        .pointer_enter(move |_| ctx.cursor.set(index))
}
