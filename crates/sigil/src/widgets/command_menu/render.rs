use crate::events::MouseButton;
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::Space;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Signal, memo, signal};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::widgets::{Layout, Widget};
use std::rc::Rc;

use crate::style::{ControlSize, Metrics, space};

use super::super::field::FIELD_GAP;
use super::super::layout_ext::LayoutOverrides;
use super::super::listbox::{ListModel, listbox};
use super::super::popup::{self, ItemSlots, item_row};
use super::super::scroll::{reveal_row, scroll};
use super::super::shared::{Activate, caller_style};
use super::super::text_field::{TextFieldBuilder, text_field};
use super::builder::CommandMenuBuilder;
use super::nav::{CommandPalette, MenuLevel, PaletteEntry, SubmitObserver};

const LEVEL_GAP: f32 = space(2);
const MAX_ROWS: usize = 12;

#[derive(Clone, Copy)]
struct Active {
    focused: Signal<bool>,
    pointed: Signal<bool>,
}

struct CommandModel {
    palette: CommandPalette,
    on_submit: Option<SubmitObserver>,
    on_cancel: Option<Activate>,
}

impl ListModel for CommandModel {
    fn cursor(&self) -> Signal<usize> {
        self.palette
            .top()
            .map_or_else(|| signal(0), |level| level.cursor)
    }

    fn len(&self) -> usize {
        self.palette
            .top()
            .map_or(0, |level| level.shown.peek().len())
    }

    fn enabled(&self, index: usize) -> bool {
        self.palette
            .top()
            .and_then(|level| level.entry_at(index).map(|entry| !entry.disabled()))
            .unwrap_or(false)
    }

    fn label(&self, index: usize) -> Option<String> {
        let level = self.palette.top()?;
        level.entry_at(index).map(|entry| entry.label().to_owned())
    }

    fn commit(&self, _index: usize) {
        self.palette.activate(self.on_submit.as_ref());
    }

    fn close(&self) {
        self.palette.ascend_or_cancel(self.on_cancel.as_ref());
    }

    fn blur(&self) {
        self.palette.reset();
    }

    fn text_input(&self) -> bool {
        true
    }
}

impl CommandMenuBuilder {
    pub(super) fn root(self) -> NodeBuilder {
        let parts = self;
        let palette = CommandPalette::new(Rc::new(parts.level.entries));
        let chrome = PanelChrome {
            active: Active {
                focused: signal(false),
                pointed: signal(false),
            },
            palette,
            panel_layout: parts.panel_layout,
            on_submit: parts.on_submit.clone(),
            placeholder: parts.placeholder,
        };
        let root = container()
            .role(Role::Menu)
            .label(theme_label(StringKey::CommandMenu))
            .layout(
                LayoutStyle::flex_row()
                    .gap(Space::px(LEVEL_GAP))
                    .align_items(AlignItems::Start),
            )
            .children_reactive(move || {
                let levels = palette.stack.get();
                let top = levels.len().saturating_sub(1);
                levels
                    .iter()
                    .enumerate()
                    .map(|(depth, level)| build_level_panel(level, depth, depth == top, &chrome))
                    .collect::<Vec<_>>()
            });
        let root = listbox(
            root,
            CommandModel {
                palette,
                on_submit: parts.on_submit,
                on_cancel: parts.on_cancel,
            },
        );
        caller_style(root, parts.style_accumulator)
    }
}

struct PanelChrome {
    active: Active,
    palette: CommandPalette,
    panel_layout: LayoutOverrides,
    on_submit: Option<SubmitObserver>,
    placeholder: String,
}

fn query_field(chrome: &PanelChrome, depth: usize) -> TextFieldBuilder {
    let (palette, active) = (chrome.palette, chrome.active);
    let on_submit = chrome.on_submit.clone();
    text_field(palette.query)
        .placeholder(chrome.placeholder.clone())
        .autofocus()
        .map_node(move |node| node.focus_in(move || active.focused.set(true)))
        .focus_lost(move || {
            active.focused.set(false);
            if !active.pointed.peek() && palette.stack.peek().len() == depth + 1 {
                palette.reset();
            }
        })
        .change(move |_| palette.refilter())
        .submit(move |_| palette.activate(on_submit.as_ref()))
}

fn rows_height(rows: usize) -> f32 {
    let item_height = Metrics::current().control_height(ControlSize::Default);
    f32::from(u16::try_from(rows).unwrap_or(u16::MAX)) * item_height
}

fn panel_layout(row_count: usize, with_field: bool) -> LayoutStyle {
    let gap = if with_field { FIELD_GAP } else { 0.0 };
    let field = if with_field {
        rows_height(1) + gap
    } else {
        0.0
    };
    LayoutStyle::flex_column()
        .height(popup::panel_height([
            rows_height(row_count.min(MAX_ROWS)),
            field,
        ]))
        .padding(Space::px(Metrics::current().overlay_padding()))
        .gap(Space::px(gap))
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
}

fn build_level_panel(
    level: &MenuLevel,
    depth: usize,
    is_top: bool,
    chrome: &PanelChrome,
) -> NodeBuilder {
    let overrides = &chrome.panel_layout;
    let layout = overrides.apply(panel_layout(level.entries.len(), is_top));
    let layout = if overrides.width_hint().is_some() {
        layout
    } else {
        layout.width(popup::PANEL_WIDTH)
    };
    let mut panel = popup::surface(Role::Menu).with(layout);
    if is_top {
        panel = panel.child(query_field(chrome, depth));
    }
    panel.child(level_rows(level, chrome))
}

fn level_rows(level: &MenuLevel, chrome: &PanelChrome) -> NodeBuilder {
    let level = level.clone();
    let row = MenuRowCtx {
        cursor: level.cursor,
        active: chrome.active,
        palette: chrome.palette,
        on_submit: chrome.on_submit.clone(),
    };
    let (count, cursor) = (level.entries.len(), level.cursor);
    let list = container()
        .layout(LayoutStyle::flex_column().align_items(AlignItems::Stretch))
        .children_reactive(move || {
            level
                .shown
                .get()
                .iter()
                .filter_map(|index| level.entries.get(*index))
                .enumerate()
                .map(|(idx, entry)| build_row(entry, idx, &row))
                .collect::<Vec<_>>()
        });
    if count <= MAX_ROWS {
        return list;
    }
    let (offset, viewport) = (signal(0.0_f32), rows_height(MAX_ROWS));
    reveal_row(offset, cursor, rows_height(1), viewport);
    scroll(list)
        .height(viewport)
        .scroll_bind(offset)
        .into_node_builder()
        .focusable(false)
}

#[derive(Clone)]
struct MenuRowCtx {
    cursor: Signal<usize>,
    active: Active,
    palette: CommandPalette,
    on_submit: Option<SubmitObserver>,
}

fn build_row(entry: &PaletteEntry, idx: usize, row: &MenuRowCtx) -> NodeBuilder {
    let disabled = entry.disabled();
    let slots = ItemSlots {
        chevron: matches!(entry, PaletteEntry::Submenu { .. }),
        shortcut: entry.detail().map(str::to_owned),
        disabled,
        ..ItemSlots::new(entry.label().to_owned())
    };
    let (cursor, active) = (row.cursor, row.active);
    let highlight =
        memo(move || (active.focused.get() || active.pointed.get()) && cursor.get() == idx);
    let click = row.clone();
    item_row(slots, highlight)
        .role(Role::MenuItem)
        .pointer_enter(move |_| {
            if !disabled {
                cursor.set(idx);
                active.pointed.set(true);
            }
        })
        .pointer_leave(move |_| active.pointed.set(false))
        .pointer_up(move |event| {
            if event.button != Some(MouseButton::Left) || disabled {
                return;
            }
            event.stop_propagation();
            click.cursor.set(idx);
            click.palette.activate(click.on_submit.as_ref());
        })
}
