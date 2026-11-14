use crate::kernel::{IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::Space;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Position};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{BORDER_WIDTH, Radius, Style, StyleAtom, Token, Tone, space};
use crate::widgets::Widget;

use super::composition::{STATUS_BAR_HEIGHT, STATUS_CHROME, kv_row};
pub use super::notice::notice_band;
use super::shared::{accessible_name, caller_style};

const TOP_BAR_HEIGHT: f32 = space(12);
const AVATAR_PX: f32 = space(7);

static TOP_CHROME: &[StyleAtom] = &[
    Token::SurfaceSubtle.background(),
    StyleAtom::BorderBottom(BORDER_WIDTH),
    Token::Border.border(),
];

/// Top bar, work area, status bar, and modal slot.
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
                        .padding(Space::Sm),
                )
                .class(Token::Surface.background())
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
                .hittable(false)
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
    let band = LayoutStyle::flex_row().fill_width();
    let band = if crate::layout::snap::cell().is_some() {
        band.min_size(0.0, height)
    } else {
        band.height(height)
    };
    container()
        .role(Role::GenericContainer)
        .layout(
            band.shrink(0.0)
                .padding_x(Space::Sm)
                .gap(Space::Sm)
                .align_items(AlignItems::Center),
        )
        .class(chrome)
        .child(child)
}

/// A two-column key/value grid.
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

/// Builder of a key hint overlay.
#[derive(Debug)]
#[must_use]
pub struct HintOverlayBuilder {
    entries: Vec<(String, String)>,
    style_accumulator: Style,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl HintOverlayBuilder {
    /// Creates an overlay with one row for each `(chord, description)` pair in `hints`.
    pub(crate) fn new(hints: &[(&str, &str)]) -> Self {
        let entries = hints
            .iter()
            .map(|(chord, desc)| ((*chord).to_owned(), (*desc).to_owned()))
            .collect();
        Self {
            entries,
            style_accumulator: Style::new(),
            layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
        }
    }

    /// Adds a row that shows the key `chord` and its `description`.
    pub fn entry(mut self, chord: impl Into<String>, description: impl Into<String>) -> Self {
        self.entries.push((chord.into(), description.into()));
        self
    }

    fn root(self) -> NodeBuilder {
        let mut root = container()
            .role(Role::GenericContainer)
            .label(theme_label(StringKey::KeyboardShortcuts))
            .layout(
                LayoutStyle::flex_column()
                    .padding(Space::Md)
                    .gap(Space::Sm)
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
                            .gap(Space::Sm)
                            .align_items(AlignItems::Center)
                            .justify_content(JustifyContent::Start),
                    )
                    .child(text(chord).class("font-mono"))
                    .child(text(desc)),
            );
        }
        caller_style(root, self.style_accumulator)
    }
}

builder_impls!([] HintOverlayBuilder: paint(style_accumulator), widget, node);

/// An overlay that lists key bindings.
pub fn hint_overlay(hints: &[(&str, &str)]) -> HintOverlayBuilder {
    HintOverlayBuilder::new(hints)
}

/// Flexible empty space.
pub fn spacer() -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
}

/// Builder of a placeholder for empty content.
#[must_use]
pub struct EmptyStateBuilder {
    title: TextContent,
    description: Option<TextContent>,
    action: Option<Box<dyn IntoNode>>,
    style_accumulator: Style,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for EmptyStateBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EmptyStateBuilder")
            .field("title", &self.title)
            .field("description", &self.description)
            .field("action", &self.action.is_some())
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish()
    }
}

/// A placeholder for empty content headed by `title`, which is also its accessible name.
pub fn empty_state(title: impl Into<TextContent>) -> EmptyStateBuilder {
    EmptyStateBuilder {
        title: title.into(),
        description: None,
        action: None,
        style_accumulator: Style::new(),
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl EmptyStateBuilder {
    /// Sets the muted text shown below the title.
    pub fn description(mut self, description: impl Into<TextContent>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the node shown below the description, such as a button.
    pub fn action(mut self, action: impl IntoNode + 'static) -> Self {
        self.action = Some(Box::new(action));
        self
    }

    fn root(self) -> NodeBuilder {
        let name = accessible_name(&self.title);
        let mut root = container()
            .role(Role::Status)
            .label(name)
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .padding(Space::Xxxl)
                    .gap(Space::Md)
                    .align_items(AlignItems::Center),
            )
            .child(
                text(self.title)
                    .class("text-lg font-medium")
                    .class(Token::OnSurface.text()),
            );
        if let Some(description) = self.description {
            root = root.child(
                text(description)
                    .class("text-sm")
                    .class(Tone::Neutral.step(11).text()),
            );
        }
        if let Some(action) = self.action {
            root = root.child(action);
        }
        caller_style(root, self.style_accumulator)
    }
}

builder_impls!([] EmptyStateBuilder: paint(style_accumulator), widget, node);

/// A section heading row.
pub fn section_label(title: impl Into<TextContent>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .shrink(0.0)
                .padding_x(Space::Sm),
        )
        .child(
            text(title)
                .class("text-xs")
                .class(Tone::Neutral.step(11).text()),
        )
}

/// A key hint row.
pub fn hint_bar(keys: impl Into<TextContent>, right: impl Into<TextContent>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(row_height())
                .shrink(0.0)
                .padding_x(Space::Sm)
                .align_items(AlignItems::Center),
        )
        .child(
            text(keys)
                .class("text-xs")
                .class(Tone::Neutral.step(11).text()),
        )
        .child(spacer())
        .child(
            text(right)
                .class("text-xs")
                .class(Tone::Primary.step(11).text()),
        )
}

/// Initials in a circle.
pub fn avatar(initials: impl Into<TextContent>) -> NodeBuilder {
    let content: TextContent = initials.into();
    let name = accessible_name(&content);
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
        .class([Radius::Full.atom(), Tone::Primary.step(9).background()])
        .child(
            text(content)
                .class("text-xs font-medium")
                .class(Token::OnAccent.text())
                .hittable(false),
        )
}

mod card;
mod disclosure;
mod list_row;
mod navigation;
mod sidebar;
mod status;

pub use card::{CardBuilder, card};
pub use disclosure::disclosure;
use list_row::row_height;
pub use list_row::{Disclosure, ListRowBuilder, RowTone, list_row};
pub(crate) use list_row::{ROW_SELECTED, ROW_SELECTED_HOVER, row_states};
pub use navigation::{breadcrumbs, pagination};
pub use sidebar::{SidebarBuilder, sidebar};
pub use status::{SkeletonBuilder, badge, skeleton, stepper};

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
