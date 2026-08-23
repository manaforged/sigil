//! GPU menu bar. A styled strip of menus. Each menu is a trigger plus
//! a dropdown that uses the same items as [`super::menu`].
//!
//! Native OS bars stay on the desktop host (`set_menu_bar`). This bar
//! is the tree you `.class()`.

use std::rc::Rc;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TopLayer, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, signal};
use sigil_style::{NodeStyleExt, Style, StyleAccess};

use super::super::interactive::interactive;
use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::BuilderMutator;
use super::builder::{MENU_BASE, MenuItem, Row, build_item, build_separator, panel_height};

const TRIGGER_H: f32 = 32.0;
const MENU_W: f32 = 220.0;

struct Slot {
    label: String,
    rows: Vec<Row>,
}

/// Builder returned by [`menu_bar`].
pub struct MenuBarBuilder {
    slots: Vec<Slot>,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// Horizontal GPU menu bar. Style the strip with `.class(...)`.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// menu_bar()
///     .class("bg-surface px-2")
///     .menu("File", [MenuItem::new("Open").select(|| {})])
/// ```
#[must_use]
pub fn menu_bar() -> MenuBarBuilder {
    MenuBarBuilder {
        slots: Vec::new(),
        mutators: Vec::new(),
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl MenuBarBuilder {
    /// Append a named menu. Click the label to open the dropdown.
    #[must_use]
    pub fn menu(
        mut self,
        label: impl Into<String>,
        items: impl IntoIterator<Item = MenuItem>,
    ) -> Self {
        self.slots.push(Slot {
            label: label.into(),
            rows: items.into_iter().map(Row::Item).collect(),
        });
        self
    }

    /// Utility classes on the strip (`bg-surface`, `px-2`, `hover:` on
    /// triggers is per-item).
    #[must_use]
    pub fn class(mut self, classes: &str) -> Self {
        self.style_accumulator = self.style_accumulator.class(classes);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let open_idx: Signal<Option<usize>> = signal(None);
        let mut bar = container()
            .role(Role::GenericContainer)
            .label("Menu bar")
            .layout(self.layout_overrides.apply(bar_layout()))
            .class("bg-surface");

        if !self.style_accumulator.is_empty() {
            bar = bar.style(self.style_accumulator);
        }

        for (index, slot) in self.slots.into_iter().enumerate() {
            bar = bar.child(slot_node(index, slot, open_idx));
        }

        let mut root = bar;
        for mutate in self.mutators {
            root = mutate(root);
        }
        root
    }
}

impl WidgetLayoutExt for MenuBarBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl StyleAccess for MenuBarBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for MenuBarBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn bar_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .height(TRIGGER_H)
        .gap(4.0)
        .align_items(AlignItems::Center)
}

fn trigger_width(label: &str) -> f32 {
    const PAD: f32 = 16.0;
    const EM: f32 = 8.0;
    PAD + (label.chars().count() as f32) * EM
}

fn slot_node(index: usize, slot: Slot, open_idx: Signal<Option<usize>>) -> NodeBuilder {
    let trigger = interactive_trigger(slot.label.clone(), index, open_idx);
    let rows = slot.rows;
    let height = panel_height(&rows);
    let width = trigger_width(&slot.label);
    container()
        .layout(LayoutStyle::block().height(TRIGGER_H).width(width))
        .child(trigger)
        .children_reactive(move || {
            if open_idx.get() == Some(index) {
                vec![slot_popup(&rows, height, open_idx)]
            } else {
                Vec::new()
            }
        })
}

fn interactive_trigger(
    label: String,
    index: usize,
    open_idx: Signal<Option<usize>>,
) -> impl IntoNode {
    let inner = container()
        .role(Role::Button)
        .label(label.clone())
        .hittable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(TRIGGER_H)
                .padding(8.0)
                .align_items(AlignItems::Center),
        )
        .pointer_up(move |event| {
            event.stop_propagation();
            if open_idx.peek() == Some(index) {
                open_idx.set(None);
            } else {
                open_idx.set(Some(index));
            }
        })
        .child(text(label).class("text-sm text-on-surface"));
    interactive(inner).class("rounded-sm hover:bg-accent-9")
}

fn slot_popup(rows: &[Row], height: f32, open_idx: Signal<Option<usize>>) -> NodeBuilder {
    let close: Rc<dyn Fn()> = Rc::new(move || open_idx.set(None));
    let mut panel = container()
        .role(Role::Menu)
        .with(TopLayer)
        .layout(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .inset_top(TRIGGER_H)
                .inset_left(0.0)
                .width(MENU_W)
                .height(height),
        )
        .class("rounded-md")
        .class("shadow-lg")
        .class(MENU_BASE);
    for row in rows {
        panel = match row {
            Row::Item(item) => panel.child(build_item(item.clone(), Rc::clone(&close))),
            Row::Separator => panel.child(build_separator()),
        };
    }
    panel
}

#[cfg(test)]
#[path = "bar_tests.rs"]
mod tests;
