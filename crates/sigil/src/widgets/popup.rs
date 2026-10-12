mod item;

use crate::kernel::{NodeBuilder, Role, TopLayer, container};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::layout::{Popup, PopupSide};
use crate::reactive::Signal;
use crate::style::{
    BORDER_WIDTH, ControlSize, DisplayKind, Metrics, OVERLAY_SHADOW, Radius, StyleAtom, Token,
    space,
};

pub(crate) use item::{ChevronDirection, ItemSlots, chevron, footer, item_row};

pub(crate) const PANEL_INSET: f32 = space(1) + 1.0;
pub(crate) const PANEL_WIDTH: f32 = space(48);
pub(crate) const POPUP_GAP: f32 = space(1);

pub(crate) static SURFACE: &[StyleAtom] = &[
    Token::Overlay.bg(),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::OverlayBorder.border(),
    StyleAtom::Shadow(OVERLAY_SHADOW),
    Radius::Md.atom(),
];

static SEPARATOR_LINE: &[StyleAtom] = &[Token::OverlayBorder.bg()];

#[must_use]
pub(crate) fn surface(role: Role) -> NodeBuilder {
    container().role(role).class(SURFACE)
}

#[must_use]
pub(crate) fn popup(role: Role, side: PopupSide) -> NodeBuilder {
    surface(role).with(TopLayer).with(Popup {
        side,
        center: false,
        sibling: false,
        gap: POPUP_GAP,
    })
}

#[must_use]
pub(crate) fn inset() -> f32 {
    Metrics::current().overlay_padding() + crate::layout::snap::rule(f32::from(BORDER_WIDTH.0))
}

#[must_use]
pub(crate) fn separator_height() -> f32 {
    Metrics::current().overlay_padding().mul_add(2.0, 1.0)
}

#[must_use]
pub(crate) fn separator() -> NodeBuilder {
    let metrics = Metrics::current();
    container()
        .role(Role::Splitter)
        .layout(
            LayoutStyle::block()
                .fill_width()
                .height(separator_height())
                .shrink(0.0)
                .padding_x(metrics.padding(ControlSize::Default))
                .padding_y(metrics.overlay_padding()),
        )
        .child(
            container()
                .layout(LayoutStyle::block().fill_width().height(1.0))
                .class(SEPARATOR_LINE),
        )
}

#[must_use]
pub(crate) fn panel_height(rows: impl IntoIterator<Item = f32>) -> f32 {
    let item = Metrics::current().control_height(ControlSize::Default);
    let body = rows.into_iter().sum::<f32>().max(item);
    inset().mul_add(2.0, body)
}

pub(crate) fn shown(show: Signal<bool>) -> &'static [StyleAtom] {
    if show.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}
