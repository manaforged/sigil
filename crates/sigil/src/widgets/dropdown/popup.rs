use std::rc::Rc;

use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use sigil_style::{DisplayKind, NodeStyleExt, StyleAtom};

use super::super::shared::{Activate, disabled_now};

pub(super) const TRIGGER_WIDTH: f32 = 200.0;
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
}

pub(super) fn build_popup(
    ctx: OptionCtx,
    options: &Rc<Vec<String>>,
    on_change: &Activate,
) -> NodeBuilder {
    let OptionCtx {
        selected,
        open,
        cursor,
        disabled,
    } = ctx;
    let n = options.len();
    #[allow(clippy::cast_precision_loss)]
    let popup_h = (n.min(POPUP_MAX_VISIBLE) as f32) * OPTION_HEIGHT;

    let mut popup = container()
        .role(Role::ListBox)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(TRIGGER_WIDTH, popup_h)
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
    };
    for (idx, label) in options.iter().enumerate() {
        popup = popup.child(build_option(
            idx,
            label.clone(),
            item_ctx,
            Rc::clone(on_change),
        ));
    }

    popup
}

pub(super) const fn popup_display(open: bool) -> &'static [StyleAtom] {
    if open {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

fn build_option(idx: usize, label: String, ctx: OptionCtx, on_change: Activate) -> NodeBuilder {
    use sigil_events::prelude::NodeEventsExt;
    container()
        .role(Role::ListBoxOption)
        .label(label.clone())
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(TRIGGER_WIDTH, OPTION_HEIGHT)
                .padding(8.0),
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
            on_change();
            ev.stop_propagation();
        })
        .pointer_enter(move |_| {
            ctx.cursor.set(idx);
        })
        .child(
            text(label)
                .layout(LayoutStyle::block().width(TRIGGER_WIDTH - 16.0))
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
