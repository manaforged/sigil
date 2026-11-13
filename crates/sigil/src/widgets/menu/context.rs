use std::rc::Rc;

use crate::events::MouseButton;
use crate::kernel::{IntoNode, NodeBuilder, container};
use crate::layout::{LayoutStyle, Position};
use crate::reactive::{Owner, Signal, signal};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string;
use crate::style::{ControlSize, Metrics};
use crate::widgets::Widget;

use super::super::popup::{PANEL_INSET, PANEL_WIDTH, PopupAnchor, PopupLayer};
use super::builder::{MenuItem, menu};

/// Wraps `target`; a right click opens a menu of `items()` at the pointer.
pub fn context_menu(
    target: impl IntoNode + 'static,
    items: impl Fn() -> Vec<MenuItem> + 'static,
) -> NodeBuilder {
    let open = signal(false);
    let pos: Signal<(f32, f32)> = signal((0.0, 0.0));
    let host = Owner::current();
    let items = Rc::new(items);
    let items_for_open = Rc::clone(&items);
    container()
        .pointer_down(move |event| {
            if event.button == Some(MouseButton::Right) && !items_for_open().is_empty() {
                pos.set((event.window_position.x, event.window_position.y));
                open.set(true);
                event.stop_propagation();
            }
        })
        .child(target)
        .child(
            {
                let layer = PopupLayer::new(open, PopupAnchor::Top);
                layer.register_escape();
                layer.register_light();
                layer
            }
            .wrap(
                container()
                    .layout(
                        LayoutStyle::block()
                            .position(Position::Absolute)
                            .inset_left(0.0)
                            .inset_top(0.0)
                            .fill_width()
                            .fill_height(),
                    )
                    .children_reactive(move || {
                        let at = pos.get();
                        if !open.get() {
                            return Vec::new();
                        }
                        let rows = items();
                        if rows.is_empty() {
                            return Vec::new();
                        }
                        vec![anchored_menu(rows, open, at, host)]
                    }),
            ),
        )
}

fn anchored_menu(
    rows: Vec<MenuItem>,
    open: Signal<bool>,
    (x, y): (f32, f32),
    host: Option<Owner>,
) -> NodeBuilder {
    let viewport = crate::app::viewport_size().peek();
    let item_height = Metrics::current().control_height(ControlSize::Default);
    let height = rows
        .iter()
        .fold(PANEL_INSET * 2.0, |total, _| total + item_height);
    let fit = |at: f32, extent: f32, size: f32| {
        if extent > 0.0 {
            at.clamp(0.0, (extent - size).max(0.0))
        } else {
            at
        }
    };
    let mut panel = menu(open)
        .label(theme_string(StringKey::ContextMenu))
        .anchor((
            fit(x, viewport.width, PANEL_WIDTH),
            fit(y, viewport.height, height),
        ));
    if let Some(host) = host {
        panel = panel.host(host);
    }
    for row in rows {
        panel = panel.item(row);
    }
    panel.into_node_builder()
}
