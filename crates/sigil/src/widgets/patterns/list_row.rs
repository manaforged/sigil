use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, TextBuilder, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Length, cols, rows};
use crate::reactive::{Signal, signal};
use crate::style::States;
use crate::style::{Paint, Radius, Token, Tone, space};
use crate::widgets::Layout;

use super::super::chip::chip;
use super::super::interactive::interactive;
use super::super::popup::{ChevronDirection, chevron};
use super::super::table::ColumnAlign;
use super::activatable::activatable;

const INDENT_COLS: f32 = 2.0;
const CELL_COLLAPSED: &str = "▸";
const CELL_EXPANDED: &str = "▾";

pub(super) fn row_height() -> f32 {
    crate::layout::snap::line(24.0)
}

/// The tree chevron slot of a list row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disclosure {
    /// The none disclosure.
    #[default]
    None,
    /// The leaf disclosure.
    Leaf,
    /// The collapsed disclosure.
    Collapsed,
    /// The expanded disclosure.
    Expanded,
}

/// Text tone of a list row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RowTone {
    /// The normal row tone.
    #[default]
    Normal,
    /// The muted row tone.
    Muted,
    /// The header row tone.
    Header,
}

impl RowTone {
    const fn title(self) -> Token {
        match self {
            Self::Normal => Token::OnSurface,
            Self::Muted | Self::Header => Token::Gray10,
        }
    }

    const fn detail(self) -> Token {
        match self {
            Self::Normal | Self::Header => Token::Gray11,
            Self::Muted => Token::Gray9,
        }
    }
}

/// One fixed-width column of a list row.
#[derive(Debug)]
pub struct ListColumn {
    /// The text.
    pub text: TextContent,
    /// The width.
    pub width: f32,
    /// The align.
    pub align: ColumnAlign,
}

impl ListColumn {
    /// Creates a list column.
    #[must_use]
    pub fn new(text: impl Into<TextContent>, width: f32) -> Self {
        Self {
            text: text.into(),
            width,
            align: ColumnAlign::End,
        }
    }

    /// Aligns the column to the start.
    #[must_use]
    pub const fn start(mut self) -> Self {
        self.align = ColumnAlign::Start;
        self
    }
}

impl From<(TextContent, f32)> for ListColumn {
    fn from((text, width): (TextContent, f32)) -> Self {
        Self::new(text, width)
    }
}

/// Optional parts of a list row.
#[derive(Default)]
pub struct ListRowSlots {
    /// The depth.
    pub depth: u16,
    /// The disclosure.
    pub disclosure: Disclosure,
    /// The leading, if any.
    pub leading: Option<Box<dyn IntoNode>>,
    /// The marker, if any.
    pub marker: Option<TextContent>,
    /// The marker tone, if any.
    pub marker_tone: Option<Tone>,
    /// The status, if any.
    pub status: Option<TextContent>,
    /// The status tone, if any.
    pub status_tone: Option<Tone>,
    /// The badge, if any.
    pub badge: Option<(TextContent, Tone)>,
    /// The detail, if any.
    pub detail: Option<TextContent>,
    /// The columns.
    pub columns: Vec<ListColumn>,
    /// Whether shrink.
    pub shrink: bool,
    /// The title width, if any.
    pub title_width: Option<f32>,
    /// The indent, if any.
    pub indent: Option<f32>,
    /// The tone.
    pub tone: RowTone,
    /// The activate, if any.
    pub activate: Option<Rc<dyn Fn()>>,
}

impl core::fmt::Debug for ListRowSlots {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ListRowSlots")
            .field("depth", &self.depth)
            .field("disclosure", &self.disclosure)
            .field("marker", &self.marker)
            .field("marker_tone", &self.marker_tone)
            .field("status", &self.status)
            .field("status_tone", &self.status_tone)
            .field("badge", &self.badge)
            .field("detail", &self.detail)
            .field("columns", &self.columns)
            .field("shrink", &self.shrink)
            .field("title_width", &self.title_width)
            .field("indent", &self.indent)
            .field("tone", &self.tone)
            .finish_non_exhaustive()
    }
}

/// Builds one list row from its slots.
#[must_use]
pub fn list_row(
    selected: bool,
    title: impl Into<TextContent>,
    mut slots: ListRowSlots,
) -> NodeBuilder {
    let mut row = row_frame(
        slots.depth,
        slots.indent.unwrap_or(INDENT_COLS),
        slots.disclosure,
    );
    if let Some(marker) = slots.marker.take() {
        row = row.child(marker_slot(marker, slots.marker_tone));
    }
    if let Some(leading) = slots.leading.take() {
        row = row.child(leading_slot(leading));
    }
    let tone = slots.tone;
    let title = title.into();
    if let TextContent::Static(name) = &title {
        row = row.label(name.clone());
    }
    row = row.child(title_text(title, &slots));
    if let Some(detail) = slots.detail.take() {
        row = row.child(detail_text(detail, tone));
    }
    if let Some(status) = slots.status.take() {
        row = row.child(status_text(status, slots.shrink, tone, slots.status_tone));
    }
    if let Some((label, badge_tone)) = slots.badge.take() {
        row = row.child(chip(label, badge_tone));
    }
    for entry in std::mem::take(&mut slots.columns) {
        row = row.child(column(entry, slots.shrink, tone));
    }
    if tone == RowTone::Header {
        return row;
    }
    let row = if selected {
        row.selected(signal(true))
            .class([Radius::Md.atom(), ROW_SELECTED.background()])
    } else {
        row_states(row, None)
    };
    match slots.activate {
        Some(activate) => activatable(row, Role::ListItem, &activate),
        None => row,
    }
}

pub(crate) const ROW_SELECTED: Token = Token::Accent5;
pub(crate) const ROW_SELECTED_HOVER: Token = Token::Accent6;

pub(crate) fn row_states(row: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let row = interactive(row.class(Radius::Md.atom()))
        .hover(|s| s.background(Token::SurfaceHover))
        .pressed(|s| s.background(Token::SurfacePressed));
    match disabled {
        Some(disabled) => row.disabled(disabled),
        None => row,
    }
    .into_node_builder()
}

fn row_frame(depth: u16, indent_cols: f32, disclosure: Disclosure) -> NodeBuilder {
    let pad = space(2);
    let mut row = container().role(Role::GenericContainer).layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(row_height())
            .padding_x(Space::px(pad))
            .gap(Space::px(pad))
            .align_items(AlignItems::Center),
    );
    if depth > 0 {
        row = row.child(
            container().layout(
                LayoutStyle::block()
                    .width(cols(indent_cols * f32::from(depth)))
                    .shrink(0.0),
            ),
        );
    }
    if matches!(disclosure, Disclosure::Collapsed | Disclosure::Expanded) {
        row = row.expanded(signal(disclosure == Disclosure::Expanded));
    }
    if disclosure != Disclosure::None {
        row = row.child(disclosure_slot(disclosure));
    }
    row
}

fn marker_slot(marker: TextContent, tone: Option<Tone>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(cols(2.0)).shrink(0.0))
        .child(
            text(marker)
                .class("text-sm")
                .class(tone.map_or(Token::Accent11, Tone::solid).text()),
        )
}

fn leading_slot(content: Box<dyn IntoNode>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .shrink(0.0)
                .align_items(AlignItems::Center),
        )
        .child(content)
}

pub(super) fn disclosure_slot(disclosure: Disclosure) -> NodeBuilder {
    let slot = container().layout(
        LayoutStyle::flex_row()
            .width(cols(INDENT_COLS))
            .shrink(0.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    );
    let mark = CellForm::<fn(ChevronDirection) -> Box<dyn IntoNode>>::new(
        |direction| Box::new(chevron(direction, space(3))),
        cell_mark,
    )
    .pick();
    match disclosure {
        Disclosure::Collapsed => slot.child(mark(ChevronDirection::Right)),
        Disclosure::Expanded => slot.child(mark(ChevronDirection::Down)),
        Disclosure::None | Disclosure::Leaf => slot,
    }
}

fn cell_mark(direction: ChevronDirection) -> Box<dyn IntoNode> {
    let glyph = match direction {
        ChevronDirection::Right => CELL_COLLAPSED,
        ChevronDirection::Down => CELL_EXPANDED,
    };
    Box::new(
        container()
            .layout(LayoutStyle::block().width(cols(1.0)).height(rows(1.0)))
            .class(Token::OnSurfaceMuted.text())
            .child(text(glyph)),
    )
}

fn title_text(title: TextContent, slots: &ListRowSlots) -> TextBuilder {
    let layout = match slots.title_width {
        Some(width) => LayoutStyle::block()
            .width(width)
            .shrink(1.0)
            .min_size(0.0, 0.0),
        None if slots.detail.is_some() => LayoutStyle::block().shrink(0.0),
        None if slots.shrink => LayoutStyle::block()
            .grow(1.0)
            .min_size(0.0, 0.0)
            .shrink(0.0)
            .max_size(Length::Percent(1.0), Length::Auto),
        None => LayoutStyle::block().grow(1.0).min_size(0.0, 0.0),
    };
    text(title)
        .ellipsize()
        .layout(layout)
        .class(if slots.tone == RowTone::Header {
            "text-xs"
        } else {
            "text-sm"
        })
        .class(slots.tone.title().text())
}

fn detail_text(detail: TextContent, tone: RowTone) -> TextBuilder {
    text(detail)
        .ellipsize()
        .class("text-xs")
        .class(tone.detail().text())
        .layout(
            LayoutStyle::block()
                .grow(1.0)
                .shrink(1.0)
                .min_size(0.0, 0.0),
        )
}

fn status_text(
    status: TextContent,
    shrink: bool,
    tone: RowTone,
    status_tone: Option<Tone>,
) -> TextBuilder {
    let node = text(status).ellipsize().class("text-xs").class(
        status_tone
            .map_or_else(|| tone.detail(), |shade| shade.token(12))
            .text(),
    );
    if shrink {
        node.layout(LayoutStyle::block().shrink(1.0).min_size(0.0, 0.0))
    } else {
        node.layout(LayoutStyle::block().shrink(0.0))
    }
}

fn column(entry: ListColumn, shrink: bool, tone: RowTone) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .width(entry.width)
                .shrink(if shrink { 1.0 } else { 0.0 })
                .min_size(0.0, 0.0)
                .justify_content(match entry.align {
                    ColumnAlign::Start => JustifyContent::Start,
                    ColumnAlign::End => JustifyContent::End,
                }),
        )
        .child(
            text(entry.text)
                .ellipsize()
                .class("text-xs")
                .class(tone.detail().text()),
        )
}
