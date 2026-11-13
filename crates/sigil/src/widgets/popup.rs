mod item;
mod layer;

use crate::kernel::{NodeBuilder, Role, TopLayer, container};
use crate::layout::LayoutStyle;
use crate::layout::Space;
use crate::layout::{Popup, PopupSide};
use crate::style::{BORDER_WIDTH, ControlSize, Layer, Metrics, StyleAtom, Token, space};
use crate::widgets::Layout;

pub(crate) use item::{ChevronDirection, ItemSlots, chevron, footer, item_row};
pub use layer::close_popup;
pub(crate) use layer::{
    PopupAnchor, PopupLayer, close_light, close_on_escape, hide, shown, shown_when,
};
pub(crate) use layer::{Stacks as PopupStacks, swap_layers};

pub(crate) const PANEL_INSET: f32 = space(1) + 1.0;
pub(crate) const PANEL_WIDTH: f32 = space(48);
pub(crate) const POPUP_GAP: f32 = space(1);

pub(crate) static SURFACE: &[StyleAtom] = Layer::Overlay.atoms();

static SEPARATOR_LINE: &[StyleAtom] = &[Token::OverlayBorder.background()];

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
    Metrics::current().overlay_padding() + crate::layout::snap::rule(f32::from(BORDER_WIDTH))
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
                .padding_x(Space::px(metrics.control_padding(ControlSize::Default)))
                .padding_y(Space::px(metrics.overlay_padding())),
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
