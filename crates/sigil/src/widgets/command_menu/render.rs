use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived, signal};
use crate::style::{Metrics, NodeStyleExt, Size, Style, space};

use super::super::layout_ext::LayoutOverrides;
use super::super::listbox::{ListModel, listbox};
use super::super::popup::{self, ItemSlots, item_row};
use super::super::shared::Activate;
use super::builder::LevelDef;
use super::nav::{
    Entry, MenuLevel, SelectObserver, activate_or_descend, ascend_or_cancel, into_runtime,
    reset_to_root,
};

const LEVEL_GAP: f32 = space(2);

#[derive(Clone, Copy)]
struct Active {
    focused: Signal<bool>,
    pointed: Signal<bool>,
}

impl Active {
    fn new() -> Self {
        Self {
            focused: signal(false),
            pointed: signal(false),
        }
    }
}

struct CommandModel {
    stack: Signal<Vec<MenuLevel>>,
    on_select: Option<SelectObserver>,
    on_cancel: Option<Activate>,
}

impl CommandModel {
    fn top(&self) -> Option<MenuLevel> {
        self.stack.peek().last().cloned()
    }
}

impl ListModel for CommandModel {
    fn cursor(&self) -> Signal<usize> {
        self.top().map_or_else(|| signal(0), |level| level.cursor)
    }

    fn len(&self) -> usize {
        self.top().map_or(0, |level| level.entries.len())
    }

    fn enabled(&self, index: usize) -> bool {
        self.top()
            .and_then(|level| level.entries.get(index).map(|entry| !entry.disabled()))
            .unwrap_or(false)
    }

    fn label(&self, index: usize) -> Option<String> {
        let level = self.top()?;
        level
            .entries
            .get(index)
            .map(|entry| entry.label().to_owned())
    }

    fn commit(&self, _index: usize) {
        activate_or_descend(self.stack, self.on_select.as_ref());
    }

    fn close(&self) {
        ascend_or_cancel(self.stack, self.on_cancel.as_ref());
    }

    fn blur(&self) {
        reset_to_root(self.stack);
    }
}

pub(super) fn into_node_builder(
    level: LevelDef,
    on_select: Option<SelectObserver>,
    on_cancel: Option<Activate>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
) -> NodeBuilder {
    let root_entries = into_runtime(level);
    let stack: Signal<Vec<MenuLevel>> = signal(vec![MenuLevel::new(root_entries)]);
    let cancel_left = on_cancel.clone();
    let select_right = on_select.clone();
    let active = Active::new();

    let root = container()
        .role(Role::Menu)
        .label("Command Menu")
        .focusable(true)
        .layout(root_layout())
        .focus_in(move || active.focused.set(true))
        .focus_out(move || active.focused.set(false))
        .key(Key::ArrowLeft, move |ev| {
            ev.stop_propagation();
            ascend_or_cancel(stack, cancel_left.as_ref());
        })
        .key(Key::ArrowRight, move |ev| {
            ev.stop_propagation();
            activate_or_descend(stack, select_right.as_ref());
        })
        .children_reactive(move || {
            stack
                .get()
                .iter()
                .map(|panel| build_level_panel(panel, active, &layout_overrides))
                .collect::<Vec<_>>()
        });
    let mut root = listbox(
        root,
        CommandModel {
            stack,
            on_select,
            on_cancel,
        },
    );
    if !style_accumulator.is_empty() {
        root = root.style(style_accumulator);
    }
    root
}

fn root_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .gap(LEVEL_GAP)
        .align_items(AlignItems::Start)
}

fn panel_layout(row_count: usize) -> LayoutStyle {
    let item_height = Metrics::current().control_height(Size::Default);
    let rows = f32::from(u16::try_from(row_count).unwrap_or(u16::MAX));
    LayoutStyle::flex_column()
        .height(popup::panel_height([rows * item_height]))
        .padding(Metrics::current().overlay_padding())
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
}

fn build_level_panel(
    level: &MenuLevel,
    active: Active,
    layout_overrides: &LayoutOverrides,
) -> NodeBuilder {
    let cursor = level.cursor;
    let layout = layout_overrides
        .clone()
        .apply(panel_layout(level.entries.len()));
    let layout = if layout_overrides.width_hint().is_some() {
        layout
    } else {
        layout.width(popup::PANEL_WIDTH)
    };
    let mut panel = popup::surface(Role::Menu).layout(layout);
    for (idx, entry) in level.entries.iter().enumerate() {
        panel = panel.child(build_row(entry, idx, cursor, active));
    }
    panel
}

fn build_row(entry: &Entry, idx: usize, cursor: Signal<usize>, active: Active) -> NodeBuilder {
    let disabled = entry.disabled();
    let slots = ItemSlots {
        chevron: matches!(entry, Entry::Submenu { .. }),
        disabled,
        ..ItemSlots::new(entry.label().to_owned())
    };
    let highlight =
        derived(move || (active.focused.get() || active.pointed.get()) && cursor.get() == idx);
    item_row(slots, highlight)
        .role(Role::MenuItem)
        .pointer_enter(move |_| {
            if !disabled {
                cursor.set(idx);
                active.pointed.set(true);
            }
        })
        .pointer_leave(move |_| active.pointed.set(false))
}
