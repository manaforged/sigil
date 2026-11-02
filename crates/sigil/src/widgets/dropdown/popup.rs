use std::rc::Rc;

use crate::kernel::{NodeBuilder, Role};
use crate::layout::snap::CellForm;
use crate::layout::{IntoPx, rows};
use crate::reactive::{Signal, derived};
use crate::style::{StyleAtom, Token};
use crate::widgets::Widget;

use super::super::combobox::list_popup;
use super::super::listbox::ListModel;
use super::super::popup::{ItemSlots, POPUP_GAP, item_row};
use super::super::shared::disabled_now;
use super::builder::IndexChange;

const TRIGGER_HEIGHT: f32 = 32.0;
const TRIGGER_ROWS: f32 = 3.0;

pub(super) fn trigger_height() -> f32 {
    CellForm::new(TRIGGER_HEIGHT, rows(TRIGGER_ROWS).into_px()).pick()
}
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

pub(super) fn build_popup(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    on_change: &IndexChange,
) -> NodeBuilder {
    list_popup(
        (ctx.trigger_height + POPUP_GAP, ctx.width),
        (options.len(), POPUP_MAX_VISIBLE),
        ctx.cursor,
        options
            .iter()
            .enumerate()
            .map(|(index, label)| build_option(index, label.clone(), ctx, Rc::clone(on_change))),
    )
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
