use std::rc::Rc;

use crate::events::prelude::{NodeEventsExt, PointerEvent};
use crate::kernel::{Cx, FocusScopeKind, IntoNode, NodeBuilder, container};
use crate::layout::PopupSide;
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Owner, OwnerId, Signal};
use crate::style::{DisplayKind, NodeStyleExt, Style, StyleAccess, StyleAtom};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::popup::PANEL_WIDTH;
use super::panel::{Row, menu_panel, panel_height, panel_layout};

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub submenu_indicator: bool,
    pub(super) select: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem")
            .field("label", &self.label)
            .field("shortcut", &self.shortcut)
            .field("disabled", &self.disabled)
            .field("submenu_indicator", &self.submenu_indicator)
            .field("select", &self.select.is_some())
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
            select: None,
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
    pub fn select(mut self, select: impl Fn() + 'static) -> Self {
        self.select = Some(Rc::new(select));
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
        let (layout, placement) = placement(self.anchor, width, height);
        let menu_label = self.label.unwrap_or_else(|| String::from("Menu"));

        let close: Rc<dyn Fn()> = Rc::new(move || open.set(false));
        let mut panel = menu_panel(self.rows, close, PopupSide::Keep, self.host)
            .mutate(|node| node.a11y.focus_scope = Some(FocusScopeKind::Popup))
            .label(menu_label)
            .layout(self.layout_overrides.apply(placement))
            .style_reactive(move || open_display(open))
            .pointer_down(PointerEvent::stop_propagation);
        if !self.style_accumulator.is_empty() {
            panel = panel.style(self.style_accumulator);
        }

        container()
            .layout(layout)
            .style_reactive(move || open_display(open))
            .pointer_down(move |event| {
                open.set(false);
                event.stop_propagation();
            })
            .child(panel)
    }
}

fn placement(anchor: Option<(f32, f32)>, width: f32, height: f32) -> (LayoutStyle, LayoutStyle) {
    if let Some((x, y)) = anchor {
        return (
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0)
                .fill_width()
                .fill_height(),
            panel_layout(y, x, width, height),
        );
    }
    (
        LayoutStyle::block().width(width).height(height),
        panel_layout(0.0, 0.0, width, height),
    )
}

impl WidgetLayoutExt for MenuBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl StyleAccess for MenuBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for MenuBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn open_display(open: Signal<bool>) -> &'static [StyleAtom] {
    if open.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
