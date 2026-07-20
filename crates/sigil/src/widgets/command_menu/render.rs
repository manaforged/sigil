//! Command menu node construction — builds the reactive panel stack
//! and wires Up/Down/Enter/Right/Esc/Left to [`super::nav`]'s action
//! functions. Panels lay out left-to-right in normal flex flow (no
//! absolute viewport insets) so the open stack cascades beside its
//! parent level.

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{FocusRing, NodeStyleExt, Size, Style, StyleAtom, Theme};

use super::super::shared::Activate;
use super::builder::LevelDef;
use super::nav::{
    Entry, MenuLevel, SelectObserver, activate_or_descend, ascend_or_cancel, into_runtime,
    move_cursor,
};

/// Panel width in logical pixels — matches the sibling `menu` widget's
/// panel width so command menus read consistently beside it.
const PANEL_WIDTH: f32 = 180.0;
/// Gap between cascading level panels.
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
                .map(build_level_panel)
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
        .fixed_size(PANEL_WIDTH, (rows * item_height).max(item_height))
        .align_items(AlignItems::Stretch)
}

fn build_level_panel(level: &MenuLevel) -> NodeBuilder {
    let cursor = level.cursor;
    let mut panel = container()
        .role(Role::Menu)
        .layout(panel_layout(level.entries.len()))
        .class("rounded-md")
        .class(PANEL_BASE);
    for (idx, entry) in level.entries.iter().enumerate() {
        panel = panel.child(build_row(entry, idx, cursor));
    }
    panel
}

fn build_row(entry: &Entry, idx: usize, cursor: Signal<usize>) -> NodeBuilder {
    let label = entry.label().to_owned();
    let disabled = entry.disabled();
    let is_submenu = matches!(entry, Entry::Submenu { .. });
    let ring_visible = derived_eq(move || cursor.get() == idx);

    let mut row = container()
        .role(Role::MenuItem)
        .label(label.clone())
        .layout(row_layout())
        .style_reactive(move || row_atoms(cursor.get() == idx, disabled))
        .focus_ring(FocusRing::new(ring_visible))
        .pointer_enter(move |_| {
            if !disabled {
                cursor.set(idx);
            }
        })
        .child(
            text(label)
                .layout(LayoutStyle::block().width(row_label_width(is_submenu)))
                .class("text-on-surface"),
        );

    if is_submenu {
        row = row.child(submenu_swatch());
    }
    row
}

fn row_layout() -> LayoutStyle {
    let theme = Theme::default();
    let item_height = theme.control_height(Size::Default).px_or_zero();
    let padding = theme.padding(Size::Default).px_or_zero();
    LayoutStyle::flex_row()
        .fixed_size(PANEL_WIDTH, item_height)
        .padding(padding)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
}

fn row_label_width(is_submenu: bool) -> f32 {
    let theme = Theme::default();
    let padding = theme.padding(Size::Default).px_or_zero();
    let interior = 2.0_f32.mul_add(-padding, PANEL_WIDTH);
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
