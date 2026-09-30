use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::style::{BORDER_WIDTH, Radius, StyleAtom, Token, space};

use super::composition::{STATUS_BAR_HEIGHT, STATUS_CHROME, kv_row};
pub use super::notice::notice_band;

const TOP_BAR_HEIGHT: f32 = space(12);

static TOP_CHROME: &[StyleAtom] = &[
    Token::SurfaceSubtle.bg(),
    StyleAtom::BorderBottom(BORDER_WIDTH),
    Token::Border.border(),
];

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
        .layout(chrome_layout(TOP_BAR_HEIGHT))
        .class(TOP_CHROME)
        .child(child)
}

fn work_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .padding(space(2)),
        )
        .class(Token::Surface.bg())
        .child(child)
}

fn status_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(chrome_layout(STATUS_BAR_HEIGHT))
        .class(STATUS_CHROME)
        .child(child)
}

fn chrome_layout(height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fill_width()
        .height(height)
        .shrink(0.0)
        .padding_x(space(2))
        .gap(space(2))
        .align_items(AlignItems::Center)
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
pub fn kv_grid(rows: &[(&'static str, &'static str)]) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width());
    let last = rows.len().saturating_sub(1);
    for (index, (key, value)) in rows.iter().enumerate() {
        let row = kv_row(*key, text(*value).class("font-mono"));
        root = root.child(if index < last {
            row.class(ROW_RULE)
        } else {
            row
        });
    }
    root
}

static ROW_RULE: &[StyleAtom] = &[
    StyleAtom::BorderBottom(BORDER_WIDTH),
    Token::Border.border(),
];

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
                    .padding(space(3))
                    .gap(space(2))
                    .align_items(AlignItems::Start)
                    .justify_content(JustifyContent::Start),
            )
            .class(super::popup::SURFACE)
            .class(OVERLAY_TEXT);

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

static OVERLAY_TEXT: &[StyleAtom] = &[Token::OnSurface.text()];

fn hint_row(chord: &str, description: &str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .gap(space(2))
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

const AVATAR_PX: f32 = space(7);

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
        .layout(
            LayoutStyle::block()
                .fixed_size(f32::from(BORDER_WIDTH.0), space(4))
                .shrink(0.0),
        )
        .class(Token::Border.bg())
}

#[must_use]
pub fn empty_state(title: impl Into<TextContent>, body: impl Into<TextContent>) -> NodeBuilder {
    let title: TextContent = title.into();
    let name = title.as_static_str().unwrap_or("Empty").to_owned();
    container()
        .role(Role::Status)
        .label(name)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(space(12))
                .gap(space(3))
                .align_items(AlignItems::Center),
        )
        .child(
            text(title)
                .class("text-lg font-medium")
                .class(Token::OnSurface.text()),
        )
        .child(text(body).class("text-sm").class(Token::Gray11.text()))
}

#[must_use]
pub fn section_label(title: impl Into<TextContent>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .padding_x(space(2)),
        )
        .child(text(title).class("text-xs").class(Token::Gray11.text()))
}

#[must_use]
pub fn hint_bar(keys: impl Into<TextContent>, right: impl Into<TextContent>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .padding_x(space(2))
                .align_items(AlignItems::Center),
        )
        .child(text(keys).class("text-xs").class(Token::Gray11.text()))
        .child(spacer())
        .child(text(right).class("text-xs").class(Token::Accent11.text()))
}

#[must_use]
pub fn card(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .padding(space(3))
                .gap(space(2))
                .fill_width(),
        )
        .class([
            Radius::Md.atom(),
            Token::Surface.bg(),
            StyleAtom::BorderWidth(BORDER_WIDTH),
            Token::Border.border(),
        ])
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
        .class([Radius::Full.atom(), Token::Accent9.bg()])
        .child(
            text(content)
                .class("text-xs font-medium")
                .class(Token::OnAccent.text())
                .hittable(false),
        )
}

mod list_row;
mod sidebar;

use list_row::row_height;
pub use list_row::{Disclosure, ListColumn, ListRowSlots, RowTone, list_row, list_row_with};
pub(crate) use list_row::{ROW_SELECTED, ROW_SELECTED_HOVER, row_states};
pub use sidebar::{sidebar, sidebar_width};

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
