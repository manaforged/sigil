use crate::events::{MouseButton, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived, signal};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{ControlSize, Metrics, Style, space};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::listbox::{ListModel, listbox};
use super::super::popup::{self, ItemSlots, item_row};
use super::super::shared::Activate;
use super::super::text_field::{TextFieldBuilder, text_field};
use super::builder::LevelDef;
use super::nav::{Entry, MenuLevel, Palette, SubmitObserver, into_runtime};

const LEVEL_GAP: f32 = space(2);
const FIELD_GAP: f32 = space(1);

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
    palette: Palette,
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

pub(super) struct MenuParts {
    pub(super) level: LevelDef,
    pub(super) on_submit: Option<SubmitObserver>,
    pub(super) on_cancel: Option<Activate>,
    pub(super) placeholder: String,
    pub(super) style_accumulator: Style,
    pub(super) layout_overrides: LayoutOverrides,
}

pub(super) fn into_node_builder(parts: MenuParts) -> NodeBuilder {
    let palette = Palette::new(into_runtime(parts.level));
    let active = Active::new();
    let chrome = PanelChrome {
        active,
        palette,
        layout_overrides: parts.layout_overrides,
        on_submit: parts.on_submit.clone(),
        placeholder: parts.placeholder,
    };
    let root = container()
        .role(Role::Menu)
        .label(theme_label(StringKey::CommandMenu))
        .layout(root_layout())
        .children_reactive(move || {
            let levels = palette.stack.get();
            let top = levels.len().saturating_sub(1);
            levels
                .iter()
                .enumerate()
                .map(|(depth, level)| build_level_panel(level, depth, depth == top, &chrome))
                .collect::<Vec<_>>()
        });
    let mut root = listbox(
        root,
        CommandModel {
            palette,
            on_submit: parts.on_submit,
            on_cancel: parts.on_cancel,
        },
    );
    if !parts.style_accumulator.is_empty() {
        root = root.style(parts.style_accumulator);
    }
    root
}

struct PanelChrome {
    active: Active,
    palette: Palette,
    layout_overrides: LayoutOverrides,
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
        .blur(move || {
            active.focused.set(false);
            if palette.stack.peek().len() == depth + 1 {
                palette.reset();
            }
        })
        .change(move |_| palette.refilter())
        .submit(move |_| palette.activate(on_submit.as_ref()))
}

fn root_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .gap(LEVEL_GAP)
        .align_items(AlignItems::Start)
}

fn panel_layout(row_count: usize, with_field: bool) -> LayoutStyle {
    let item_height = Metrics::current().control_height(ControlSize::Default);
    let rows = f32::from(u16::try_from(row_count).unwrap_or(u16::MAX));
    let field = if with_field {
        item_height + FIELD_GAP
    } else {
        0.0
    };
    LayoutStyle::flex_column()
        .height(popup::panel_height([rows * item_height, field]))
        .padding(Metrics::current().overlay_padding())
        .gap(if with_field { FIELD_GAP } else { 0.0 })
        .shrink(0.0)
        .align_items(AlignItems::Stretch)
}

fn build_level_panel(
    level: &MenuLevel,
    depth: usize,
    is_top: bool,
    chrome: &PanelChrome,
) -> NodeBuilder {
    let overrides = &chrome.layout_overrides;
    let layout = overrides
        .clone()
        .apply(panel_layout(level.entries.len(), is_top));
    let layout = if overrides.width_hint().is_some() {
        layout
    } else {
        layout.width(popup::PANEL_WIDTH)
    };
    let panel = popup::surface(Role::Menu).with(layout);
    let panel = if is_top {
        panel.child(query_field(chrome, depth))
    } else {
        panel
    };
    panel.child(level_rows(level, chrome))
}

fn level_rows(level: &MenuLevel, chrome: &PanelChrome) -> NodeBuilder {
    let level = level.clone();
    let row = RowCtx {
        cursor: level.cursor,
        active: chrome.active,
        palette: chrome.palette,
        on_submit: chrome.on_submit.clone(),
    };
    container()
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
        })
}

#[derive(Clone)]
struct RowCtx {
    cursor: Signal<usize>,
    active: Active,
    palette: Palette,
    on_submit: Option<SubmitObserver>,
}

fn build_row(entry: &Entry, idx: usize, row: &RowCtx) -> NodeBuilder {
    let disabled = entry.disabled();
    let slots = ItemSlots {
        chevron: matches!(entry, Entry::Submenu { .. }),
        disabled,
        ..ItemSlots::new(entry.label().to_owned())
    };
    let (cursor, active) = (row.cursor, row.active);
    let highlight =
        derived(move || (active.focused.get() || active.pointed.get()) && cursor.get() == idx);
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
