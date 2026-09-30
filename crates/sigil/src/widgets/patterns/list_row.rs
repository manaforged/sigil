use crate::kernel::{NodeBuilder, Role, TextBuilder, TextContent, container, text};
use crate::layout::{AlignItems, Dimension, JustifyContent, LayoutBuilderExt, LayoutStyle, cols};
use crate::reactive::{Signal, signal};
use crate::style::{Paint, Radius, StateVariants, Token, space};

use super::super::interactive::interactive;
use super::super::popup::{ChevronDirection, chevron};
use super::super::table::ColAlign;

const INDENT_COLS: f32 = 2.0;

pub(super) fn row_height() -> f32 {
    crate::layout::snap::line(24.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disclosure {
    #[default]
    None,
    Leaf,
    Collapsed,
    Expanded,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowTone {
    #[default]
    Normal,
    Muted,
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

pub struct ListColumn {
    pub text: TextContent,
    pub width: f32,
    pub align: ColAlign,
}

impl ListColumn {
    #[must_use]
    pub fn new(text: impl Into<TextContent>, width: f32) -> Self {
        Self {
            text: text.into(),
            width,
            align: ColAlign::End,
        }
    }

    #[must_use]
    pub const fn start(mut self) -> Self {
        self.align = ColAlign::Start;
        self
    }
}

impl From<(TextContent, f32)> for ListColumn {
    fn from((text, width): (TextContent, f32)) -> Self {
        Self::new(text, width)
    }
}

#[derive(Default)]
pub struct ListRowSlots {
    pub depth: u16,
    pub disclosure: Disclosure,
    pub marker: Option<TextContent>,
    pub status: Option<TextContent>,
    pub detail: Option<TextContent>,
    pub columns: Vec<ListColumn>,
    pub shrink: bool,
    pub title_width: Option<f32>,
    pub tone: RowTone,
}

#[must_use]
pub fn list_row(
    selected: bool,
    title: impl Into<TextContent>,
    detail: impl Into<TextContent>,
) -> NodeBuilder {
    let detail: TextContent = detail.into();
    let empty = matches!(&detail, TextContent::Static(value) if value.is_empty());
    let slots = ListRowSlots {
        status: (!empty).then_some(detail),
        ..ListRowSlots::default()
    };
    list_row_with(selected, title, slots)
}

#[must_use]
pub fn list_row_with(
    selected: bool,
    title: impl Into<TextContent>,
    mut slots: ListRowSlots,
) -> NodeBuilder {
    let mut row = row_frame(slots.depth, slots.disclosure);
    if let Some(marker) = slots.marker.take() {
        row = row.child(
            container()
                .layout(LayoutStyle::block().width(cols(2.0)).shrink(0.0))
                .child(text(marker).class("text-sm").class(Token::Accent11.text())),
        );
    }
    let tone = slots.tone;
    row = row.child(title_text(title.into(), &slots));
    if let Some(detail) = slots.detail.take() {
        row = row.child(detail_text(detail, tone));
    }
    if let Some(status) = slots.status.take() {
        row = row.child(status_text(status, slots.shrink, tone));
    }
    for entry in std::mem::take(&mut slots.columns) {
        row = row.child(column(entry, slots.shrink, tone));
    }
    if tone == RowTone::Header {
        return row;
    }
    if selected {
        return row
            .selected(signal(true))
            .class([Radius::Md.atom(), ROW_SELECTED.bg()]);
    }
    row_states(row, None)
}

pub(crate) const ROW_SELECTED: Token = Token::Accent5;
pub(crate) const ROW_SELECTED_HOVER: Token = Token::Accent6;

pub(crate) fn row_states(row: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let row = interactive(row.class(Radius::Md.atom()))
        .hover(|s| s.bg(Token::SurfaceHover))
        .pressed(|s| s.bg(Token::SurfacePressed));
    match disabled {
        Some(disabled) => row.disabled(disabled),
        None => row,
    }
    .into_node_builder()
}

fn row_frame(depth: u16, disclosure: Disclosure) -> NodeBuilder {
    let pad = space(2);
    let mut row = container().role(Role::GenericContainer).layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(row_height())
            .padding_x(pad)
            .gap(pad)
            .align_items(AlignItems::Center),
    );
    if depth > 0 {
        row = row.child(indent(depth));
    }
    if matches!(disclosure, Disclosure::Collapsed | Disclosure::Expanded) {
        row = row.expanded(signal(disclosure == Disclosure::Expanded));
    }
    if disclosure != Disclosure::None {
        row = row.child(disclosure_slot(disclosure));
    }
    row
}

fn indent(depth: u16) -> NodeBuilder {
    container().layout(
        LayoutStyle::block()
            .width(cols(INDENT_COLS * f32::from(depth)))
            .shrink(0.0),
    )
}

fn disclosure_slot(disclosure: Disclosure) -> NodeBuilder {
    let slot = container().layout(
        LayoutStyle::flex_row()
            .width(cols(INDENT_COLS))
            .shrink(0.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    );
    match disclosure {
        Disclosure::Collapsed => slot.child(chevron(ChevronDirection::Right, space(3))),
        Disclosure::Expanded => slot.child(chevron(ChevronDirection::Down, space(3))),
        Disclosure::None | Disclosure::Leaf => slot,
    }
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
            .max_size(Dimension::Percent(1.0), Dimension::Auto),
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

fn status_text(status: TextContent, shrink: bool, tone: RowTone) -> TextBuilder {
    let node = text(status)
        .ellipsize()
        .class("text-xs")
        .class(tone.detail().text());
    if shrink {
        node.layout(LayoutStyle::block().shrink(1.0).min_size(0.0, 0.0))
    } else {
        node
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
                    ColAlign::Start => JustifyContent::Start,
                    ColAlign::End => JustifyContent::End,
                }),
        )
        .child(
            text(entry.text)
                .ellipsize()
                .class("text-xs")
                .class(tone.detail().text()),
        )
}
