use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, derived};
use crate::style::{DisplayKind, NodeStyleExt, StyleAtom, Tone};

use super::shared::label;
use super::text_field::TextFieldBuilder;
use super::tooltip::{TooltipBuilder, tooltip as tooltip_widget};

const TOP_BAR_HEIGHT: f32 = 48.0;
const STATUS_BAR_HEIGHT: f32 = 28.0;
const FIELD_GAP: f32 = 4.0;
const KV_GRID_ROW_GAP: f32 = 4.0;
const KV_GRID_COLUMN_GAP: f32 = 12.0;

#[must_use]
pub fn app_shell(
    top: impl IntoNode + 'static,
    work: impl IntoNode + 'static,
    status: impl IntoNode + 'static,
    modal: Option<impl IntoNode + 'static>,
) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill())
        .child(top_section(top))
        .child(work_section(work))
        .child(status_section(status));
    if let Some(overlay) = modal {
        root = root.child(modal_overlay(overlay));
    }
    root
}

fn top_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_row().fill_width().height(TOP_BAR_HEIGHT))
        .child(child)
}

fn work_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().grow(1.0))
        .child(child)
}

fn status_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(STATUS_BAR_HEIGHT),
        )
        .child(child)
}

fn modal_overlay(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .fill(),
        )
        .child(child)
}

#[must_use]
pub fn field(label_text: impl Into<TextContent>, control: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().gap(FIELD_GAP))
        .child(label(label_text))
        .child(control)
        .on_added(|cx, id| {
            let Some(node) = cx.get(id) else { return };
            let [label_id, control_id, ..] = *node.children() else {
                return;
            };
            if let Some(target) = cx.get_mut(control_id) {
                target.a11y.labelled_by = Some(label_id);
            }
        })
}

#[must_use]
pub fn validated_field(
    label_text: impl Into<TextContent>,
    control: TextFieldBuilder,
    error: Signal<Option<String>>,
) -> NodeBuilder {
    let message: Signal<String> = derived(move || error.get().unwrap_or_default());
    let visible: Signal<bool> = derived(move || error.get().is_some());
    let invalid_signal: Signal<bool> = derived(move || error.get().is_some());
    let control = control.invalid_when(invalid_signal);
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().gap(FIELD_GAP))
        .child(label(label_text))
        .child(control)
        .child(
            text(message)
                .role(Role::Label)
                .class("text-error-12 text-xs font-medium")
                .style_reactive(move || {
                    if visible.get() {
                        ERROR_VISIBLE
                    } else {
                        ERROR_HIDDEN
                    }
                }),
        )
}

static ERROR_VISIBLE: &[StyleAtom] = &[];
static ERROR_HIDDEN: &[StyleAtom] = &[StyleAtom::Display(DisplayKind::None)];

#[must_use]
pub fn kv_grid(rows: &[(&'static str, &'static str)]) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().gap(KV_GRID_ROW_GAP));
    for (key, value) in rows {
        root = root.child(kv_grid_row(key, value));
    }
    root
}

fn kv_grid_row(key: &'static str, value: &'static str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_row().fill_width().gap(KV_GRID_COLUMN_GAP))
        .child(label(key))
        .child(text(value).class("font-mono"))
}

#[must_use]
pub fn tooltip(target: impl IntoNode + 'static, hint: impl Into<String>) -> TooltipBuilder {
    tooltip_widget(hint).child(target)
}

pub type HintEntry<'a> = (&'a str, &'a str);

pub struct HintOverlayBuilder {
    entries: Vec<(String, String)>,
}

impl HintOverlayBuilder {
    #[must_use]
    pub fn new(hints: &[HintEntry<'_>]) -> Self {
        let entries = hints
            .iter()
            .map(|(chord, desc)| ((*chord).to_owned(), (*desc).to_owned()))
            .collect();
        Self { entries }
    }

    #[must_use]
    pub fn entry(mut self, chord: impl Into<String>, description: impl Into<String>) -> Self {
        self.entries.push((chord.into(), description.into()));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let mut root = container()
            .role(Role::GenericContainer)
            .label("Keyboard shortcuts")
            .layout(
                LayoutStyle::flex_column()
                    .padding(12.0)
                    .gap(6.0)
                    .align_items(AlignItems::Start)
                    .justify_content(JustifyContent::Start),
            )
            .class("rounded-md")
            .class(OVERLAY_BASE);

        for (chord, desc) in self.entries {
            root = root.child(hint_row(&chord, &desc));
        }
        root
    }
}

impl IntoNode for HintOverlayBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

static OVERLAY_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::TextThemeRef("on-surface"),
];

fn hint_row(chord: &str, description: &str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Start),
        )
        .child(text(chord.to_owned()).class("font-mono"))
        .child(text(description.to_owned()))
}

#[must_use]
pub fn hint_overlay(hints: &[HintEntry<'_>]) -> HintOverlayBuilder {
    HintOverlayBuilder::new(hints)
}

const AVATAR_PX: f32 = 28.0;

#[must_use]
pub fn spacer() -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
}

#[must_use]
pub fn hairline() -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::block().fixed_size(1.0, 16.0).shrink(0.0))
        .class("bg-gray-6")
}

#[must_use]
pub fn empty_state(title: impl Into<TextContent>, body: impl Into<TextContent>) -> NodeBuilder {
    container()
        .role(Role::Status)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(48.0)
                .gap(10.0)
                .align_items(AlignItems::Center),
        )
        .child(text(title).class("text-lg font-medium text-on-surface"))
        .child(text(body).class("text-sm text-gray-11"))
}

#[must_use]
pub fn section_label(title: impl Into<TextContent>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .padding_x(8.0),
        )
        .child(text(title).class("text-xs text-gray-11"))
}

#[must_use]
pub fn hint_bar(keys: impl Into<TextContent>, right: impl Into<TextContent>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .padding_x(16.0)
                .align_items(AlignItems::Center),
        )
        .child(text(keys).class("text-xs text-gray-11"))
        .child(spacer())
        .child(text(right).class("text-xs text-accent-11"))
}

#[must_use]
pub fn notice_band(body: impl Into<TextContent>, tone: Tone) -> NodeBuilder {
    let class = match tone {
        Tone::Error => "text-error-11",
        Tone::Warn => "text-warn-11",
        Tone::Success => "text-success-11",
        Tone::Info | Tone::Primary | Tone::Neutral => "text-gray-11",
    };
    container()
        .role(Role::Status)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(8.0)
                .padding_y(4.0),
        )
        .child(text(body).ellipsize().class(class))
}

#[must_use]
pub fn card(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .padding(12.0)
                .gap(8.0)
                .fill_width(),
        )
        .class("rounded-md bg-surface border border-gray-6")
        .child(child)
}

#[must_use]
pub fn avatar(initials: impl Into<TextContent>) -> NodeBuilder {
    let content: TextContent = initials.into();
    let name = match &content {
        TextContent::Static(value) => value.clone(),
        _ => String::from("avatar"),
    };
    container()
        .role(Role::Image)
        .label(name)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(AVATAR_PX, AVATAR_PX)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center)
                .shrink(0.0),
        )
        .class("rounded-full bg-accent-9")
        .child(
            text(content)
                .class("text-xs font-medium text-on-accent")
                .hittable(false),
        )
}

mod list_row;
mod sidebar;

use list_row::row_height;
pub use list_row::{ListRowSlots, list_row, list_row_with};
pub use sidebar::{sidebar, sidebar_width};

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
