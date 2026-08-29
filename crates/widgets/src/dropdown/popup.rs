use std::rc::Rc;

use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_style::{NodeStyleExt, Px, StyleAtom};

use super::super::shared::disabled_now;
use super::builder::IndexChange;

pub(super) const TRIGGER_HEIGHT: f32 = 32.0;
pub(super) const OPTION_HEIGHT: f32 = 28.0;
pub(super) const POPUP_MAX_VISIBLE: usize = 6;

pub(super) static SURFACE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
pub(super) static OPTION_ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static OPTION_REST: &[StyleAtom] = &[];
static POPUP: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("field"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderToneStep(6),
];

const POPUP_PAD: f32 = 4.0;
const POPUP_GAP: f32 = 4.0;

#[derive(Clone, Copy)]
pub(super) struct OptionCtx {
    pub(super) selected: sigil_reactive::Signal<usize>,
    pub(super) open: sigil_reactive::Signal<bool>,
    pub(super) cursor: sigil_reactive::Signal<usize>,
    pub(super) disabled: Option<sigil_reactive::Signal<bool>>,
    pub(super) width: Option<f32>,
    pub(super) trigger_height: f32,
}

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
        trigger_height,
    } = ctx;
    let n = options.len();
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    let popup_h = (n.min(POPUP_MAX_VISIBLE) as f32) * OPTION_HEIGHT;

    let frame_h = 2.0f32.mul_add(POPUP_PAD, popup_h) + 2.0;
    let popup = container()
        .role(Role::ListBox)
        .with(sigil_kernel::TopLayer)
        .layout(
            sized(LayoutStyle::flex_column(), width, frame_h)
                .position(Position::Absolute)
                .inset_top(trigger_height + POPUP_GAP)
                .inset_left(0.0)
                .padding(POPUP_PAD),
        )
        .class("rounded-md")
        .class("shadow-lg")
        .class(POPUP);

    let item_ctx = OptionCtx {
        selected,
        open,
        cursor,
        disabled,
        width,
        trigger_height,
    };
    attach_options(popup, options, item_ctx, on_change, popup_h)
}

fn attach_options(
    popup: NodeBuilder,
    options: &Rc<Vec<String>>,
    item_ctx: OptionCtx,
    on_change: &IndexChange,
    popup_h: f32,
) -> NodeBuilder {
    let n = options.len();
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
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

fn build_option(idx: usize, label: String, ctx: OptionCtx, on_change: IndexChange) -> NodeBuilder {
    use sigil_events::prelude::NodeEventsExt;
    container()
        .role(Role::ListBoxOption)
        .label(label.clone())
        .layout(
            sized(LayoutStyle::flex_row(), ctx.width, OPTION_HEIGHT)
                .padding(8.0)
                .overflow_x(Overflow::Hidden),
        )
        .class("rounded-sm")
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
        OPTION_REST
    }
}
