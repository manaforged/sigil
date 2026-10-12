use std::rc::Rc;

use crate::events::{NodeEventsExt, PointerEvent};
use crate::kernel::{Cx, FocusScopeKind, IntoNode, NodeBuilder, container};
use crate::layout::PopupSide;
use crate::layout::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Owner, OwnerId, Signal};
use crate::style::{PaintTarget, Style, StyleSlot};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::popup::{self, PANEL_WIDTH};
use super::panel::{Row, menu_panel, panel_height, panel_layout};

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub disabled: bool,
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

    #[must_use]
    pub fn shortcut(mut self, hint: impl Into<String>) -> Self {
        self.shortcut = Some(hint.into());
        self
    }

    #[must_use]
    pub const fn disabled(mut self, yes: bool) -> Self {
        self.disabled = yes;
        self
    }

    #[must_use]
    pub const fn submenu(mut self, yes: bool) -> Self {
        self.submenu_indicator = yes;
        self
    }

    #[must_use]
    pub fn activate(mut self, on_activate: impl Fn() + 'static) -> Self {
        self.on_activate = Some(Rc::new(on_activate));
        self
    }
}

pub struct MenuBuilder {
    open: Signal<bool>,
    host: Option<OwnerId>,
    rows: Vec<Row>,
    label: Option<String>,
    anchor: Option<(f32, f32)>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn menu(open: Signal<bool>) -> MenuBuilder {
    MenuBuilder::new(open)
}

impl MenuBuilder {
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

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub const fn host(mut self, owner: OwnerId) -> Self {
        self.host = Some(owner);
        self
    }

    #[must_use]
    pub const fn anchor(mut self, x: f32, y: f32) -> Self {
        self.anchor = Some((x, y));
        self
    }

    #[must_use]
    pub fn item(mut self, item: MenuItem) -> Self {
        self.rows.push(Row::Item(item));
        self
    }

    #[must_use]
    pub fn separator(mut self) -> Self {
        self.rows.push(Row::Separator);
        self
    }

    pub(crate) fn into_node_builder(self) -> NodeBuilder {
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
        let menu_label = self.label.unwrap_or_else(|| String::from("Menu"));
        let hooks = self.layout_overrides.hooks();

        let close: Rc<dyn Fn()> = Rc::new(move || open.set(false));
        let mut panel = menu_panel(self.rows, close, PopupSide::Keep, self.host)
            .mutate(|node| node.a11y.focus_scope = Some(FocusScopeKind::Popup))
            .label(menu_label)
            .with(self.layout_overrides.apply(placement))
            .style_reactive(move || popup::shown(open))
            .pointer_down(PointerEvent::stop_propagation);
        if !self.style_accumulator.is_empty() {
            panel = panel.style(self.style_accumulator);
        }

        let root = container()
            .layout(layout)
            .style_reactive(move || popup::shown(open))
            .pointer_down(move |event| {
                open.set(false);
                event.stop_propagation();
            })
            .child(panel);
        hooks.finish(root)
    }
}

impl WidgetLayoutExt for MenuBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl PaintTarget for MenuBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl IntoNode for MenuBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
