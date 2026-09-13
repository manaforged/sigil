use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived_eq, signal};
use crate::style::{FocusRing, NodeStyleExt, Size, Style, StyleAtom, Theme};

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::Activate;
use super::builder::LevelDef;
use super::nav::{
    Entry, MenuLevel, SelectObserver, activate_or_descend, ascend_or_cancel, into_runtime,
    move_cursor,
};

const LEVEL_GAP: f32 = 8.0;
const SUBMENU_INDICATOR_SIZE: f32 = 8.0;

static PANEL_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static ROW_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static ROW_CURSOR: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static ROW_DISABLED: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];
static SUBMENU_BG: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

pub(super) fn into_node_builder(
    level: LevelDef,
    on_select: Option<SelectObserver>,
    on_cancel: Option<Activate>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
) -> NodeBuilder {
    let root_entries = into_runtime(level);
    let stack: Signal<Vec<MenuLevel>> = signal(vec![MenuLevel::new(root_entries)]);

    let cancel_esc = on_cancel.clone();
    let cancel_left = on_cancel;
    let select_enter = on_select.clone();
    let select_right = on_select;

    let mut root = container()
        .role(Role::Menu)
        .label("Command Menu")
        .focusable(true)
        .layout(root_layout())
        .key(Key::Escape, move |ev| {
            ev.stop_propagation();
            ascend_or_cancel(stack, cancel_esc.as_ref());
        })
        .key(Key::ArrowLeft, move |ev| {
            ev.stop_propagation();
            ascend_or_cancel(stack, cancel_left.as_ref());
        })
        .key(Key::ArrowUp, move |ev| {
            ev.stop_propagation();
            move_cursor(stack, false);
        })
        .key(Key::ArrowDown, move |ev| {
            ev.stop_propagation();
            move_cursor(stack, true);
        })
        .key(Key::Enter, move |ev| {
            ev.stop_propagation();
            activate_or_descend(stack, select_enter.as_ref());
        })
        .key(Key::ArrowRight, move |ev| {
            ev.stop_propagation();
            activate_or_descend(stack, select_right.as_ref());
        })
        .children_reactive(move || {
            stack
                .get()
                .iter()
                .map(|panel| build_level_panel(panel, &layout_overrides))
                .collect::<Vec<_>>()
        });

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
    let theme = Theme::default();
    let item_height = theme.control_height(Size::Default).px_or_zero();
    let rows = f32::from(u16::try_from(row_count).unwrap_or(u16::MAX));
    LayoutStyle::flex_column()
        .height((rows * item_height).max(item_height))
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
}

fn build_level_panel(level: &MenuLevel, layout_overrides: &LayoutOverrides) -> NodeBuilder {
    let cursor = level.cursor;
    let mut panel = container()
        .role(Role::Menu)
        .layout(
            layout_overrides
                .clone()
                .apply(panel_layout(level.entries.len())),
        )
        .class("rounded-md")
        .class(PANEL_BASE);
    let panel_width = layout_overrides.width_hint();
    for (idx, entry) in level.entries.iter().enumerate() {
        panel = panel.child(build_row(entry, idx, cursor, panel_width));
    }
    panel
}

fn build_row(
    entry: &Entry,
    idx: usize,
    cursor: Signal<usize>,
    panel_width: Option<f32>,
) -> NodeBuilder {
    let label = entry.label().to_owned();
    let disabled = entry.disabled();
    let is_submenu = matches!(entry, Entry::Submenu { .. });
    let ring_visible = derived_eq(move || cursor.get() == idx);

    let mut row = container()
        .role(Role::MenuItem)
        .label(label.clone())
        .layout(row_layout(panel_width))
        .style_reactive(move || row_atoms(cursor.get() == idx, disabled))
        .focus_ring(FocusRing::new(ring_visible))
        .pointer_enter(move |_| {
            if !disabled {
                cursor.set(idx);
            }
        })
        .child(
            text(label)
                .layout(label_layout(is_submenu, panel_width))
                .class("text-on-surface"),
        );

    if is_submenu {
        row = row.child(submenu_swatch());
    }
    row
}

fn row_layout(panel_width: Option<f32>) -> LayoutStyle {
    let theme = Theme::default();
    let item_height = theme.control_height(Size::Default).px_or_zero();
    let padding = theme.padding(Size::Default).px_or_zero();
    let layout = LayoutStyle::flex_row()
        .height(item_height)
        .shrink(0.0)
        .padding(padding)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween);
    match panel_width {
        Some(width) => layout.width(width),
        None => layout,
    }
}

fn label_layout(is_submenu: bool, panel_width: Option<f32>) -> LayoutStyle {
    panel_width.map_or_else(
        || LayoutStyle::block().grow(1.0),
        |width| LayoutStyle::block().width(row_label_width(is_submenu, width)),
    )
}

fn row_label_width(is_submenu: bool, panel_width: f32) -> f32 {
    let theme = Theme::default();
    let padding = theme.padding(Size::Default).px_or_zero();
    let interior = 2.0_f32.mul_add(-padding, panel_width);
    let reserved = if is_submenu { 20.0 } else { 0.0 };
    (interior - reserved).max(40.0)
}

fn row_atoms(is_cursor: bool, disabled: bool) -> &'static [StyleAtom] {
    if disabled {
        ROW_DISABLED
    } else if is_cursor {
        ROW_CURSOR
    } else {
        ROW_REST
    }
}

fn submenu_swatch() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(SUBMENU_INDICATOR_SIZE, SUBMENU_INDICATOR_SIZE))
        .class("rounded-sm")
        .class(SUBMENU_BG)
}
