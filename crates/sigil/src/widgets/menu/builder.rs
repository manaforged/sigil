use std::rc::Rc;

use crate::animation_runtime::presets;
use crate::events::prelude::{Key, MouseButton, NodeEventsExt, PointerEvent};
use crate::kernel::{Cx, FocusScopeKind, IntoNode, NodeBuilder, Role, TopLayer, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, signal};
use crate::style::{
    DisplayKind, NodeStyleExt, Px, Size, Style, StyleAccess, StyleAtom, Theme, TransitionId,
};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::BuilderMutator;

const MENU_TRANSITION: TransitionId = presets::SMOOTH;

pub(crate) const PANEL_INSET: f32 = 5.0;

const SEPARATOR_HEIGHT: f32 = 1.0;

pub(super) static MENU_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::BorderWidth(Px(1)),
    StyleAtom::BorderThemeRef("border"),
    StyleAtom::Transition(MENU_TRANSITION),
];

static ITEM_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];

static ITEM_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];

static SEPARATOR_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

static DISABLED_DIM: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub submenu_indicator: bool,
    select: Option<Rc<dyn Fn()>>,
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

#[derive(Clone)]
pub(super) enum Row {
    Item(MenuItem),
    Separator,
}

pub struct MenuBuilder {
    open: Signal<bool>,
    rows: Vec<Row>,
    label: Option<String>,
    anchor: Option<(f32, f32)>,
    mutators: Vec<BuilderMutator>,
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
            rows: Vec::new(),
            label: None,
            anchor: None,
            mutators: Vec::new(),
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
        let panel_h = panel_height(&self.rows);
        let pinned_width = self.layout_overrides.width_hint();
        let (layout, placement) = placement(self.anchor, pinned_width, panel_h);
        let menu_label = self.label.unwrap_or_else(|| String::from("Menu"));

        let mut panel = container()
            .role(Role::Menu)
            .focusable(true)
            .mutate(|node| node.a11y.focus_scope = Some(FocusScopeKind::Popup))
            .with(TopLayer)
            .label(menu_label)
            .layout(self.layout_overrides.apply(placement))
            .class("rounded-md")
            .class("shadow-lg")
            .class(MENU_BASE)
            .style_reactive(move || open_display(open))
            .pointer_down(PointerEvent::stop_propagation);

        let close: Rc<dyn Fn()> = Rc::new(move || open.set(false));
        for row in self.rows {
            panel = match row {
                Row::Item(item) => panel.child(build_item(item, Rc::clone(&close))),
                Row::Separator => panel.child(build_separator()),
            };
        }

        if !self.style_accumulator.is_empty() {
            panel = panel.style(self.style_accumulator);
        }

        let mut root = container()
            .layout(layout)
            .style_reactive(move || open_display(open))
            .pointer_down(move |event| {
                open.set(false);
                event.stop_propagation();
            })
            .key(Key::Escape, move |event| {
                open.set(false);
                event.stop_propagation();
            })
            .child(panel);

        for mutate in self.mutators {
            root = mutate(root);
        }
        root
    }
}

fn placement(
    anchor: Option<(f32, f32)>,
    width: Option<f32>,
    height: f32,
) -> (LayoutStyle, LayoutStyle) {
    if let Some((x, y)) = anchor {
        return (
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0)
                .fill_width()
                .fill_height(),
            panel_layout(y, x, height),
        );
    }
    let root = LayoutStyle::block().height(height);
    let root = match width {
        Some(width) => root.width(width),
        None => root.fill_width(),
    };
    (root, panel_layout(0.0, 0.0, height))
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

pub(super) fn panel_height(rows: &[Row]) -> f32 {
    let item_height = Theme::default().control_height(Size::Default).px_or_zero();
    let height = rows
        .iter()
        .map(|row| match row {
            Row::Item(_) => item_height,
            Row::Separator => SEPARATOR_HEIGHT,
        })
        .sum::<f32>()
        .max(item_height);
    PANEL_INSET.mul_add(2.0, height)
}

fn panel_layout(top: f32, left: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_top(top)
        .inset_left(left)
        .height(height)
        .padding(PANEL_INSET - 1.0)
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
        .justify_content(JustifyContent::Start)
}

fn wire_activate(
    node: NodeBuilder,
    select: Option<Rc<dyn Fn()>>,
    close: Rc<dyn Fn()>,
) -> NodeBuilder {
    let activate = Rc::new(move || {
        if let Some(select) = &select {
            select();
        }
        close();
    });
    let pointer_activate = Rc::clone(&activate);
    node.pointer_up(move |event| {
        if event.button != Some(MouseButton::Left) {
            return;
        }
        event.stop_propagation();
        pointer_activate();
    })
    .key(Key::Enter, move |event| {
        event.stop_propagation();
        activate();
    })
}

pub(super) fn build_item(item: MenuItem, close: Rc<dyn Fn()>) -> NodeBuilder {
    let MenuItem {
        label,
        shortcut,
        disabled,
        submenu_indicator,
        select,
    } = item;

    let hover = signal(false);
    let mut node = container()
        .role(Role::MenuItem)
        .label(label.clone())
        .focusable(!disabled)
        .hittable(true)
        .layout(item_layout())
        .class(ITEM_BASE)
        .style_reactive(move || row_atoms(hover.get(), disabled))
        .pointer_enter(move |_| {
            if !disabled {
                hover.set(true);
            }
        })
        .pointer_leave(move |_| {
            hover.set(false);
        });
    if !disabled {
        node = wire_activate(node, select, close);
    }

    node = node.child(
        text(label)
            .layout(LayoutStyle::block().grow(1.0))
            .class("text-on-surface"),
    );

    if let Some(hint) = shortcut {
        node = node.child(text(hint).class("text-sm text-gray-11"));
    }

    if submenu_indicator {
        node = node.child(submenu_swatch());
    }

    node
}

fn item_layout() -> LayoutStyle {
    let theme = Theme::default();
    let item_height = theme.control_height(Size::Default).px_or_zero();
    let row_padding_x = theme.padding(Size::Default).px_or_zero();
    let row_gap = theme.gap(Size::Default).px_or_zero();
    LayoutStyle::flex_row()
        .fill_width()
        .height(item_height)
        .shrink(0.0)
        .padding_x(row_padding_x)
        .gap(row_gap)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
}

const fn row_atoms(hover: bool, disabled: bool) -> &'static [StyleAtom] {
    if disabled {
        DISABLED_DIM
    } else if hover {
        ITEM_HOVER
    } else {
        ITEM_BASE
    }
}

fn submenu_swatch() -> impl IntoNode {
    text("\u{25B8}")
        .layout(LayoutStyle::block())
        .class("text-xs")
        .class("text-on-surface")
}

pub(super) fn build_separator() -> NodeBuilder {
    container()
        .role(Role::Splitter)
        .layout(LayoutStyle::block().height(SEPARATOR_HEIGHT).padding(0.0))
        .class(SEPARATOR_BASE)
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
