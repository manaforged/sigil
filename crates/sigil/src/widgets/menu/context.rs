use crate::events::{MouseButton, NodeEventsExt};
use crate::kernel::{IntoNode, NodeBuilder, TopLayer, container};
use crate::layout::ViewportOverlay;
use crate::layout::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Owner, OwnerId, Signal, signal};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string;
use crate::style::{ControlSize, Metrics, NodeStyleExt};

use super::super::popup::{PANEL_INSET, PANEL_WIDTH};
use super::builder::{MenuItem, menu};

#[must_use]
pub fn context_menu(
    target: impl IntoNode + 'static,
    items: impl Fn() -> Vec<MenuItem> + 'static,
) -> NodeBuilder {
    let open = signal(false);
    let pos: Signal<(f32, f32)> = signal((0.0, 0.0));
    let host = Owner::current();
    container()
        .pointer_down(move |event| {
            if event.button == Some(MouseButton::Right) {
                pos.set((event.window_position.x, event.window_position.y));
                open.set(true);
                event.stop_propagation();
            }
        })
        .child(target)
        .child(
            container()
                .with(TopLayer)
                .with(ViewportOverlay)
                .layout(
                    LayoutStyle::block()
                        .position(Position::Absolute)
                        .inset_left(0.0)
                        .inset_top(0.0)
                        .fill_width()
                        .fill_height(),
                )
                .visible_when(open)
                .children_reactive(move || {
                    if !open.get() {
                        return Vec::new();
                    }
                    let rows = items();
                    if rows.is_empty() {
                        return Vec::new();
                    }
                    vec![anchored_menu(rows, open, pos.get(), host)]
                }),
        )
}

fn anchored_menu(
    rows: Vec<MenuItem>,
    open: Signal<bool>,
    (x, y): (f32, f32),
    host: Option<OwnerId>,
) -> NodeBuilder {
    let viewport = crate::app::viewport_size().peek();
    let item_height = Metrics::current().control_height(ControlSize::Default);
    let height = rows
        .iter()
        .fold(PANEL_INSET * 2.0, |total, _| total + item_height);
    let x = if viewport.width > 0.0 {
        x.clamp(0.0, (viewport.width - PANEL_WIDTH).max(0.0))
    } else {
        x
    };
    let y = if viewport.height > 0.0 {
        y.clamp(0.0, (viewport.height - height).max(0.0))
    } else {
        y
    };
    let mut panel = menu(open)
        .label(theme_string(StringKey::ContextMenu))
        .anchor(x, y);
    if let Some(host) = host {
        panel = panel.host(host);
    }
    for row in rows {
        panel = panel.item(row);
    }
    panel.into_node_builder()
}
