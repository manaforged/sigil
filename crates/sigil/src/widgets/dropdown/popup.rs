use std::rc::Rc;

use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_style::{DisplayKind, NodeStyleExt, StyleAtom};

use super::super::shared::disabled_now;
use super::builder::IndexChange;

pub(super) const TRIGGER_HEIGHT: f32 = 40.0;
pub(super) const OPTION_HEIGHT: f32 = 36.0;
pub(super) const POPUP_MAX_VISIBLE: usize = 6;

pub(super) static SURFACE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
pub(super) static SURFACE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
pub(super) static OPTION_ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

#[derive(Clone, Copy)]
pub(super) struct OptionCtx {
    pub(super) selected: sigil_reactive::Signal<usize>,
    pub(super) open: sigil_reactive::Signal<bool>,
    pub(super) cursor: sigil_reactive::Signal<usize>,
    pub(super) disabled: Option<sigil_reactive::Signal<bool>>,
    /// Trigger width, or `None` when the trigger fills its parent. The
    /// popup and its rows follow it either way, so the menu stays flush
    /// with the control that opened it.
    pub(super) width: Option<f32>,
}

/// Apply an explicit width, or fill the containing block when the trigger
/// is filling its parent (in which case no pixel width is known here).
fn sized(base: LayoutStyle, width: Option<f32>, height: f32) -> LayoutStyle {
    match width {
        Some(px) => base.width(px).height(height),
        None => base.fill_width().height(height),
    }
}

pub(super) fn build_popup(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    on_change: &IndexChange,
) -> NodeBuilder {
    let OptionCtx {
        selected,
        open,
        cursor,
        disabled,
        width,
    } = ctx;
    let n = options.len();
    #[allow(clippy::cast_precision_loss)]
    let popup_h = (n.min(POPUP_MAX_VISIBLE) as f32) * OPTION_HEIGHT;

    let popup = container()
        .role(Role::ListBox)
        // Top layer: without this the popup paints in tree order and any
        // later sibling (the next settings row) draws over it.
        .with(sigil_kernel::TopLayer)
        .layout(
            sized(LayoutStyle::flex_column(), width, popup_h)
                .position(Position::Absolute)
                .inset_top(TRIGGER_HEIGHT)
                .inset_left(0.0),
        )
        .class("rounded-md")
        .class(SURFACE_HOVER)
        .style_reactive(move || popup_display(open.get()));

    let item_ctx = OptionCtx {
        selected,
        open,
        cursor,
        disabled,
        width,
    };
    attach_options(popup, options, item_ctx, on_change, popup_h)
}

/// Attach the option rows to `popup`, wrapped in a scroll when they
/// overflow it.
///
/// `<= POPUP_MAX_VISIBLE` options fit the box exactly and stay a plain
/// child (unchanged). More than that clip and scroll, so the overflow is
/// reachable instead of painting past the popup onto whatever sits beneath.
fn attach_options(
    popup: NodeBuilder,
    options: &Rc<Vec<String>>,
    item_ctx: OptionCtx,
    on_change: &IndexChange,
    popup_h: f32,
) -> NodeBuilder {
    let n = options.len();
    #[allow(clippy::cast_precision_loss)]
    let extent = n as f32 * OPTION_HEIGHT;
    let mut list = container().layout(sized(LayoutStyle::flex_column(), item_ctx.width, extent));
    for (idx, label) in options.iter().enumerate() {
        list = list.child(build_option(
            idx,
            label.clone(),
            item_ctx,
            Rc::clone(on_change),
        ));
    }
    if n > POPUP_MAX_VISIBLE {
        popup.child(
            super::super::scroll::scroll(list, extent)
                .height(popup_h)
                .width(item_ctx.width.unwrap_or(200.0)),
        )
    } else {
        popup.child(list)
    }
}

pub(super) const fn popup_display(open: bool) -> &'static [StyleAtom] {
    if open {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

fn build_option(idx: usize, label: String, ctx: OptionCtx, on_change: IndexChange) -> NodeBuilder {
    use sigil_events::prelude::NodeEventsExt;
    container()
        .role(Role::ListBoxOption)
        .label(label.clone())
        .layout(
            sized(LayoutStyle::flex_row(), ctx.width, OPTION_HEIGHT)
                .padding(8.0)
                // Clip labels wider than the row at the popup's edge
                // instead of painting past it (matches trigger_label).
                .overflow_x(Overflow::Hidden),
        )
        .style_reactive(move || option_atoms(idx, ctx.selected.get(), ctx.cursor.get()))
        .pointer_up(move |ev| {
            if ev.is_captured() {
                return;
            }
            if disabled_now(ctx.disabled) {
                return;
            }
            ctx.selected.set(idx);
            ctx.open.set(false);
            on_change(idx);
            ev.stop_propagation();
        })
        .pointer_enter(move |_| {
            ctx.cursor.set(idx);
        })
        .child(
            text(label)
                .layout(LayoutStyle::block().fill_width())
                .class("text-on-surface"),
        )
}

pub(super) fn option_atoms(idx: usize, selected: usize, cursor: usize) -> &'static [StyleAtom] {
    if idx == selected {
        OPTION_ACTIVE
    } else if idx == cursor {
        SURFACE_HOVER
    } else {
        SURFACE
    }
}
