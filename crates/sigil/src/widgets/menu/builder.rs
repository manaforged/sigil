use std::rc::Rc;

use crate::events::PointerEvent;
use crate::kernel::{Cx, FocusScopeKind, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::PopupSide;
use crate::layout::{LayoutStyle, Position};
use crate::reactive::{Owner, Signal};
use crate::style::Style;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string_once;
use crate::widgets::Widget;

use super::super::layout_ext::LayoutOverrides;
use super::super::popup::{self, PANEL_WIDTH, PopupAnchor, PopupLayer};
use super::super::shared::caller_style;
use super::panel::{Row, menu_panel, panel_height, panel_layout};

/// One menu entry.
#[derive(Clone)]
pub struct MenuItem {
    /// The label.
    pub label: String,
    /// The shortcut, if any.
    pub shortcut: Option<String>,
    /// Whether disabled.
    pub disabled: bool,
    /// Whether submenu indicator.
    pub submenu_indicator: bool,
    pub(super) on_activate: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem")
            .field("label", &self.label)
            .field("shortcut", &self.shortcut)
            .field("disabled", &self.disabled)
            .field("submenu_indicator", &self.submenu_indicator)
            .field("on_activate", &self.on_activate.is_some())
            .finish()
    }
}

impl MenuItem {
    /// Creates a menu item.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            disabled: false,
            submenu_indicator: false,
            on_activate: None,
        }
    }

    /// Sets the shortcut.
    #[must_use]
    pub fn shortcut(mut self, hint: impl Into<String>) -> Self {
        self.shortcut = Some(hint.into());
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub const fn disabled(mut self, yes: bool) -> Self {
        self.disabled = yes;
        self
    }

    /// Sets the submenu.
    #[must_use]
    pub const fn submenu(mut self, yes: bool) -> Self {
        self.submenu_indicator = yes;
        self
    }

    /// Sets the action the item runs.
    #[must_use]
    pub fn activate(mut self, on_activate: impl Fn() + 'static) -> Self {
        self.on_activate = Some(Rc::new(on_activate));
        self
    }
}

/// Builder of a popup menu.
#[derive(Debug)]
pub struct MenuBuilder {
    open: Signal<bool>,
    host: Option<Owner>,
    rows: Vec<Row>,
    label: Option<String>,
    anchor: Option<(f32, f32)>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A popup menu.
#[must_use]
pub fn menu(open: Signal<bool>) -> MenuBuilder {
    MenuBuilder::new(open)
}

impl MenuBuilder {
    /// Creates a menu builder.
    #[must_use]
    pub fn new(open: Signal<bool>) -> Self {
        Self {
            open,
            host: Owner::current(),
            rows: Vec::new(),
            label: None,
            anchor: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the host.
    #[must_use]
    pub const fn host(mut self, owner: Owner) -> Self {
        self.host = Some(owner);
        self
    }

    /// Sets the anchor.
    #[must_use]
    pub const fn anchor(mut self, x: f32, y: f32) -> Self {
        self.anchor = Some((x, y));
        self
    }

    /// Adds an item.
    #[must_use]
    pub fn item(mut self, item: MenuItem) -> Self {
        self.rows.push(Row::Item(item));
        self
    }

    /// Applies the separator option.
    #[must_use]
    pub fn separator(mut self) -> Self {
        self.rows.push(Row::Separator);
        self
    }

    pub(crate) fn into_node_builder(mut self) -> NodeBuilder {
        let open = self.open;
        let height = panel_height(&self.rows);
        let width = self.layout_overrides.width_hint().unwrap_or(PANEL_WIDTH);
        let (layout, placement) = match self.anchor {
            Some((x, y)) => (
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_top(0.0)
                    .inset_left(0.0)
                    .fill_width()
                    .fill_height(),
                panel_layout(y, x, width, height),
            ),
            None => (
                LayoutStyle::block().width(width).height(height),
                panel_layout(0.0, 0.0, width, height),
            ),
        };
        let overrides = std::mem::take(&mut self.layout_overrides);
        let menu_label = self
            .label
            .unwrap_or_else(|| theme_string_once(StringKey::Menu));

        let close: Rc<dyn Fn()> = Rc::new(move || open.set(false));
        let panel = menu_panel(self.rows, close, PopupSide::Keep, self.host)
            .mutate(|node| node.a11y.focus_scope = Some(FocusScopeKind::Popup))
            .label(menu_label)
            .with(placement)
            .style_reactive(move || popup::shown(open))
            .pointer_down(PointerEvent::stop_propagation);
        let panel = overrides.finish(caller_style(panel, self.style_accumulator));

        let layer = PopupLayer::new(open, PopupAnchor::Inline);
        layer.register_escape();
        layer.register_light();
        layer
            .dismiss_outside(container().layout(layout))
            .style_reactive(move || popup::shown(open))
            .child(panel)
    }
}

builder_impls!([] MenuBuilder: paint(style_accumulator), widget);

impl IntoNode for MenuBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
