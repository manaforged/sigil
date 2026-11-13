use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, Role, TextBuilder, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Length, cols};
use crate::reactive::{Reactive, Signal, memo};
use crate::style::{Paint, Radius, States, Style, StyleAtom, Token, Tone, space};
use crate::widgets::Widget;

use super::super::chip::chip;
use super::super::popup::{ChevronDirection, chevron};
use super::super::shared::{Activate, caller_style};
use super::super::table::ColumnAlign;

const INDENT_COLS: f32 = 2.0;
const CELL_COLLAPSED: &str = "▸";
const CELL_EXPANDED: &str = "▾";

pub(super) fn row_height() -> f32 {
    crate::layout::snap::line(24.0)
}

/// The tree chevron slot of a list row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Disclosure {
    /// No chevron and no slot for one.
    #[default]
    None,
    /// An empty slot, so a leaf lines up with its siblings.
    Leaf,
    /// A chevron that points right.
    Collapsed,
    /// A chevron that points down.
    Expanded,
}

/// Text tone of a list row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RowTone {
    /// Body text.
    #[default]
    Normal,
    /// Dimmed text, for an inactive row.
    Muted,
    /// A small dimmed heading above a group of rows.
    Header,
}

static SELECTED_INK: &[StyleAtom] = &[Token::OnSurface.text()];
static NORMAL_DETAIL_INK: &[StyleAtom] = &[Tone::Neutral.step(11).text()];
static MUTED_DETAIL_INK: &[StyleAtom] = &[Tone::Neutral.step(9).text()];

impl RowTone {
    const fn title(self) -> Token {
        match self {
            Self::Normal => Token::OnSurface,
            Self::Muted | Self::Header => Tone::Neutral.step(10),
        }
    }

    const fn detail_atoms(self) -> &'static [StyleAtom] {
        match self {
            Self::Normal | Self::Header => NORMAL_DETAIL_INK,
            Self::Muted => MUTED_DETAIL_INK,
        }
    }
}

struct Column {
    text: TextContent,
    width: f32,
    align: ColumnAlign,
}

/// Builds a list row.
#[must_use]
pub struct ListRowBuilder {
    title: TextContent,
    selected: Option<Signal<bool>>,
    depth: u16,
    disclosure: Option<Signal<Disclosure>>,
    leading: Option<Box<dyn IntoNode>>,
    marker: Option<TextContent>,
    marker_tone: Option<Tone>,
    status: Option<TextContent>,
    status_tone: Option<Tone>,
    badge: Option<(TextContent, Tone)>,
    detail: Option<TextContent>,
    columns: Vec<Column>,
    shrink: bool,
    title_width: Option<f32>,
    indent: f32,
    tone: RowTone,
    activate: Option<Activate>,
    style_accumulator: Style,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for ListRowBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ListRowBuilder")
            .field("title", &self.title)
            .field("selected", &self.selected)
            .field("depth", &self.depth)
            .field("disclosure", &self.disclosure)
            .field("tone", &self.tone)
            .finish_non_exhaustive()
    }
}

/// One row of a list: a title, with optional marker, icon, detail, status, badge, and columns.
pub fn list_row(title: impl Into<TextContent>) -> ListRowBuilder {
    ListRowBuilder {
        title: title.into(),
        selected: None,
        depth: 0,
        disclosure: None,
        leading: None,
        marker: None,
        marker_tone: None,
        status: None,
        status_tone: None,
        badge: None,
        detail: None,
        columns: Vec::new(),
        shrink: false,
        title_width: None,
        indent: INDENT_COLS,
        tone: RowTone::Normal,
        activate: None,
        style_accumulator: Style::new(),
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl ListRowBuilder {
    /// Draws the row with the selected fill.
    pub fn selected(mut self, selected: impl Reactive<bool>) -> Self {
        self.selected = Some(selected.into_signal());
        self
    }

    /// Indents the row by `depth` levels of a tree.
    pub const fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Sets the width of one level of indent, in character cells; the default is 2.
    pub const fn indent(mut self, cells: f32) -> Self {
        self.indent = cells;
        self
    }

    /// Shows the tree chevron before the title.
    pub fn disclosure(mut self, disclosure: impl Reactive<Disclosure>) -> Self {
        self.disclosure = Some(disclosure.into_signal());
        self
    }

    /// Shows a short mark, such as a bullet, in a two-cell slot before the title.
    pub fn marker(mut self, marker: impl Into<TextContent>) -> Self {
        self.marker = Some(marker.into());
        self
    }

    /// Colors the marker; the default is the accent.
    pub const fn marker_tone(mut self, tone: Tone) -> Self {
        self.marker_tone = Some(tone);
        self
    }

    /// Shows a node, such as an icon, before the title.
    pub fn leading(mut self, leading: impl IntoNode + 'static) -> Self {
        self.leading = Some(Box::new(leading));
        self
    }

    /// Shows secondary text after the title; it takes the free width.
    pub fn detail(mut self, detail: impl Into<TextContent>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Shows short text at the end of the row.
    pub fn status(mut self, status: impl Into<TextContent>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Colors the status text.
    pub const fn status_tone(mut self, tone: Tone) -> Self {
        self.status_tone = Some(tone);
        self
    }

    /// Shows a chip at the end of the row.
    pub fn badge(mut self, label: impl Into<TextContent>, tone: Tone) -> Self {
        self.badge = Some((label.into(), tone));
        self
    }

    /// Adds a column of `width` pixels at the end of the row.
    pub fn column(mut self, text: impl Into<TextContent>, width: f32, align: ColumnAlign) -> Self {
        self.columns.push(Column {
            text: text.into(),
            width,
            align,
        });
        self
    }

    /// Lets the status and the columns shrink when the row is narrow.
    pub const fn shrink_status(mut self) -> Self {
        self.shrink = true;
        self
    }

    /// Sets the width of the title in pixels; without it the title takes the free width, or its own width when a detail is set.
    pub const fn title_width(mut self, width: f32) -> Self {
        self.title_width = Some(width);
        self
    }

    /// Sets the text tone; a `Header` row has no hover, selection, or activation.
    pub const fn tone(mut self, tone: RowTone) -> Self {
        self.tone = tone;
        self
    }

    /// Makes the row a list item that runs `activate` when the user activates it.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let mut row = row_frame(self.depth, self.indent, self.disclosure);
        if let Some(marker) = self.marker.take() {
            row = row.child(marker_slot(marker, self.marker_tone));
        }
        if let Some(leading) = self.leading.take() {
            row = row.child(leading_slot(leading));
        }
        let tone = self.tone;
        if let TextContent::Static(name) = &self.title {
            row = row.label(name.clone());
        }
        row = row.child(title_text(&self));
        if let Some(detail) = self.detail.take() {
            row = row.child(detail_text(detail, tone, self.selected));
        }
        if let Some(status) = self.status.take() {
            row = row.child(status_text(
                status,
                self.shrink,
                tone,
                self.selected,
                self.status_tone,
            ));
        }
        if let Some((label, badge_tone)) = self.badge.take() {
            row = row.child(chip(label, badge_tone));
        }
        for entry in std::mem::take(&mut self.columns) {
            row = row.child(column(entry, self.shrink, tone, self.selected));
        }
        if tone == RowTone::Header {
            return caller_style(row, self.style_accumulator);
        }
        let row = match self.selected {
            Some(sel) => row_states(row.selected(sel), None)
                .style_reactive(move || if sel.get() { ROW_SELECTED_BG } else { &[] }),
            None => row_states(row, None),
        };
        let row = match self.activate {
            Some(activate) => row.role(Role::ListItem).activate_with(activate),
            None => row,
        };
        caller_style(row, self.style_accumulator)
    }
}

impl From<ListRowBuilder> for NodeBuilder {
    fn from(row: ListRowBuilder) -> Self {
        row.into_node_builder()
    }
}

crate::widgets::builder_impls!(
    [] ListRowBuilder: paint(style_accumulator),
    states(style_accumulator),
    widget,
    node(pub)
);

pub(crate) const ROW_SELECTED: Token = Tone::Primary.step(5);
pub(crate) const ROW_SELECTED_HOVER: Token = Tone::Primary.step(6);
static ROW_SELECTED_BG: &[StyleAtom] = &[Radius::Md.atom(), ROW_SELECTED.background()];

pub(crate) fn row_states(row: NodeBuilder, disabled: Option<Signal<bool>>) -> NodeBuilder {
    let row = row
        .class(Radius::Md.atom())
        .hover(|s| s.background(Token::SurfaceHover))
        .pressed(|s| s.background(Token::SurfacePressed));
    match disabled {
        Some(disabled) => row.disabled(disabled),
        None => row,
    }
}

fn row_frame(depth: u16, indent_cols: f32, disclosure: Option<Signal<Disclosure>>) -> NodeBuilder {
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
    if let Some(disc) = disclosure {
        let current = disc.peek();
        if matches!(current, Disclosure::Collapsed | Disclosure::Expanded) {
            row = row.expanded(memo(move || disc.get() == Disclosure::Expanded));
        }
        if current != Disclosure::None {
            row = row.child(disclosure_slot_reactive(disc));
        }
    }
    row
}

fn marker_slot(marker: TextContent, tone: Option<Tone>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().width(cols(2.0)).shrink(0.0))
        .child(
            text(marker).class("text-sm").class(
                tone.map_or_else(|| Tone::Primary.step(11), Tone::solid)
                    .text(),
            ),
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

fn slot_container() -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .width(cols(INDENT_COLS))
            .shrink(0.0)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    )
}

fn disclosure_mark(direction: ChevronDirection) -> NodeBuilder {
    CellForm::<fn(ChevronDirection) -> NodeBuilder>::new(
        |dir| NodeBuilder::wrapping(chevron(dir, space(3))),
        cell_mark,
    )
    .pick()(direction)
}

pub(super) fn disclosure_slot(disclosure: Disclosure) -> NodeBuilder {
    let slot = slot_container();
    match disclosure {
        Disclosure::Collapsed => slot.child(disclosure_mark(ChevronDirection::Right)),
        Disclosure::Expanded => slot.child(disclosure_mark(ChevronDirection::Down)),
        Disclosure::None | Disclosure::Leaf => slot,
    }
}

fn disclosure_slot_reactive(disclosure: Signal<Disclosure>) -> NodeBuilder {
    if crate::layout::snap::cell().is_some() {
        return slot_container().child(cell_mark_frame(move || match disclosure.get() {
            Disclosure::Collapsed => CELL_COLLAPSED,
            Disclosure::Expanded => CELL_EXPANDED,
            Disclosure::None | Disclosure::Leaf => "",
        }));
    }
    let size = space(3);
    slot_container().children_reactive(move || match disclosure.get() {
        Disclosure::Collapsed => vec![NodeBuilder::wrapping(chevron(
            ChevronDirection::Right,
            size,
        ))],
        Disclosure::Expanded => vec![NodeBuilder::wrapping(chevron(ChevronDirection::Down, size))],
        Disclosure::None | Disclosure::Leaf => Vec::new(),
    })
}

fn cell_mark(direction: ChevronDirection) -> NodeBuilder {
    let glyph = match direction {
        ChevronDirection::Right => CELL_COLLAPSED,
        ChevronDirection::Down => CELL_EXPANDED,
    };
    cell_mark_frame(glyph)
}

fn cell_mark_frame(glyph: impl Into<TextContent>) -> NodeBuilder {
    let extent = row_height();
    container()
        .layout(LayoutStyle::block().width(extent).height(extent))
        .class(Token::OnSurfaceMuted.text())
        .child(text(glyph))
}

fn title_text(slots: &ListRowBuilder) -> TextBuilder {
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
    text(slots.title.clone())
        .ellipsize()
        .layout(layout)
        .class(if slots.tone == RowTone::Header {
            "text-xs"
        } else {
            "text-sm"
        })
        .class(slots.tone.title().text())
}

fn apply_secondary_ink(
    node: TextBuilder,
    tone: RowTone,
    selected: Option<Signal<bool>>,
) -> TextBuilder {
    let rest = tone.detail_atoms();
    match selected {
        Some(sel) => node.style_reactive(move || if sel.get() { SELECTED_INK } else { rest }),
        None => node.class(rest),
    }
}

fn detail_text(detail: TextContent, tone: RowTone, selected: Option<Signal<bool>>) -> TextBuilder {
    apply_secondary_ink(text(detail).ellipsize().class("text-xs"), tone, selected).layout(
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
    selected: Option<Signal<bool>>,
    status_tone: Option<Tone>,
) -> TextBuilder {
    let node = text(status).ellipsize().class("text-xs");
    let node = match status_tone {
        Some(shade) => node.class(shade.step(12).text()),
        None => apply_secondary_ink(node, tone, selected),
    };
    if shrink {
        node.layout(LayoutStyle::block().shrink(1.0).min_size(0.0, 0.0))
    } else {
        node.layout(LayoutStyle::block().shrink(0.0))
    }
}

fn column(
    entry: Column,
    shrink: bool,
    tone: RowTone,
    selected: Option<Signal<bool>>,
) -> NodeBuilder {
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
        .child(apply_secondary_ink(
            text(entry.text).ellipsize().class("text-xs"),
            tone,
            selected,
        ))
}
