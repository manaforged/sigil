use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::{theme_label, theme_string};
use crate::style::{BORDER_WIDTH, Radius, StyleAtom, Token, space};

use super::composition::{STATUS_BAR_HEIGHT, STATUS_CHROME, kv_row};
pub use super::notice::notice_band;

const TOP_BAR_HEIGHT: f32 = space(12);
const AVATAR_PX: f32 = space(7);

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
        .child(chrome_section(TOP_BAR_HEIGHT, TOP_CHROME, top))
        .child(
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
                .child(work),
        )
        .child(chrome_section(
            crate::layout::snap::ruled(STATUS_BAR_HEIGHT),
            STATUS_CHROME,
            status,
        ));
    if let Some(overlay) = modal {
        root = root.child(
            container()
                .role(Role::GenericContainer)
                .layout(
                    LayoutStyle::flex_column()
                        .position(Position::Absolute)
                        .fill(),
                )
                .child(overlay),
        );
    }
    root
}

fn chrome_section(
    height: f32,
    chrome: &'static [StyleAtom],
    child: impl IntoNode + 'static,
) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(height)
                .shrink(0.0)
                .padding_x(space(2))
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .class(chrome)
        .child(child)
}

#[must_use]
pub fn kv_grid<K, V>(rows: impl IntoIterator<Item = (K, V)>) -> NodeBuilder
where
    K: Into<TextContent>,
    V: Into<TextContent>,
{
    let rows: Vec<(K, V)> = rows.into_iter().collect();
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width());
    let last = rows.len().saturating_sub(1);
    for (index, (key, value)) in rows.into_iter().enumerate() {
        let row = kv_row(key, text(value).class("font-mono"));
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

static OVERLAY_TEXT: &[StyleAtom] = &[Token::OnSurface.text()];

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
            .label(theme_label(StringKey::KeyboardShortcuts))
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
            root = root.child(
                container()
                    .role(Role::GenericContainer)
                    .layout(
                        LayoutStyle::flex_row()
                            .gap(space(2))
                            .align_items(AlignItems::Center)
                            .justify_content(JustifyContent::Start),
                    )
                    .child(text(chord).class("font-mono"))
                    .child(text(desc)),
            );
        }
        root
    }
}

impl IntoNode for HintOverlayBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

#[must_use]
pub fn hint_overlay(hints: &[HintEntry<'_>]) -> HintOverlayBuilder {
    HintOverlayBuilder::new(hints)
}

#[must_use]
pub fn spacer() -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
}

#[must_use]
pub fn empty_state(title: impl Into<TextContent>, body: impl Into<TextContent>) -> NodeBuilder {
    let title: TextContent = title.into();
    let name = title
        .as_static_str()
        .map_or_else(|| theme_string(StringKey::Empty), str::to_owned);
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
pub fn avatar(initials: impl Into<TextContent>) -> NodeBuilder {
    let content: TextContent = initials.into();
    let name = match &content {
        TextContent::Static(value) => value.clone(),
        _ => theme_string(StringKey::Avatar),
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

mod activatable;
mod card;
mod list_row;
mod sidebar;

pub use card::{CardBuilder, card};
use list_row::row_height;
pub use list_row::{Disclosure, ListColumn, ListRowSlots, RowTone, list_row, list_row_with};
pub(crate) use list_row::{ROW_SELECTED, ROW_SELECTED_HOVER, row_states};
pub use sidebar::{SidebarBuilder, sidebar};

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
