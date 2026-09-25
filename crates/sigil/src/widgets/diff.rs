use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::prelude::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, Overflow};
use crate::style::{BORDER_WIDTH, NodeStyleExt, Style, StyleAccess, StyleAtom, Token, space};

use super::button::button;
use super::shared::label;

mod patch;
mod rows;

use patch::{Kind, Pair, pairs, starts};
use rows::{source_row, split_row, unified_rows, weights};

const GUTTER_W: f32 = 56.0;
pub(super) const ROW_INSET: f32 = space(2);

static ACTIVE_CHOICE: &[StyleAtom] = &[Token::Gray12.text(), Token::Gray4.bg()];
static IDLE_CHOICE: &[StyleAtom] = &[Token::Gray10.text()];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLayout {
    Split,
    Unified,
}

impl DiffLayout {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::Unified => "unified",
        }
    }
}

fn change_run(pairs: &[Pair], index: usize) -> usize {
    pairs.get(index..).map_or(0, |rest| {
        rest.iter()
            .take_while(|item| item.kind == Kind::Change)
            .count()
    })
}

fn text_row(body: String, class: &'static str, tone: Token) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fill_width().padding_x(ROW_INSET))
        .child(
            label(body)
                .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
                .class(class)
                .class(tone.text()),
        )
}

fn overflow_row() -> NodeBuilder {
    text_row(
        "… expand tool output to see the remaining changes".to_string(),
        "text-xs",
        Token::Gray10,
    )
}

type LayoutPick = Rc<dyn Fn(DiffLayout)>;

pub struct DiffBuilder {
    patch: String,
    path: Option<String>,
    status: Option<String>,
    layout: DiffLayout,
    max_rows: usize,
    pick: Option<LayoutPick>,
    style_accumulator: Style,
}

#[must_use]
pub fn diff(patch: impl Into<String>) -> DiffBuilder {
    DiffBuilder {
        patch: patch.into(),
        path: None,
        status: None,
        layout: DiffLayout::Split,
        max_rows: usize::MAX,
        pick: None,
        style_accumulator: Style::new(),
    }
}

impl DiffBuilder {
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    #[must_use]
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    #[must_use]
    pub const fn layout(mut self, layout: DiffLayout) -> Self {
        self.layout = layout;
        self
    }

    #[must_use]
    pub const fn max_rows(mut self, rows: usize) -> Self {
        self.max_rows = rows;
        self
    }

    #[must_use]
    pub fn choose(mut self, pick: impl Fn(DiffLayout) + 'static) -> Self {
        self.pick = Some(Rc::new(pick));
        self
    }

    fn header(&self, added: usize, removed: usize) -> NodeBuilder {
        let mut row = container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .padding_x(ROW_INSET)
                    .min_size(0.0, 0.0)
                    .align_items(AlignItems::Center)
                    .gap(space(2)),
            )
            .child(
                label(self.path.clone().unwrap_or_default())
                    .ellipsize()
                    .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
                    .class("text-xs font-mono font-bold")
                    .class(Token::Gray12.text()),
            )
            .child(
                label(format!("+{added}"))
                    .class("text-xs font-mono")
                    .class(Token::Success11.text()),
            )
            .child(
                label(format!("−{removed}"))
                    .class("text-xs font-mono")
                    .class(Token::Error11.text()),
            );
        for choice in [DiffLayout::Split, DiffLayout::Unified] {
            let tone = if choice == self.layout {
                ACTIVE_CHOICE
            } else {
                IDLE_CHOICE
            };
            row = row.child(match self.pick.clone() {
                Some(pick) => button(choice.name())
                    .ghost()
                    .activate(move || pick(choice))
                    .class("text-xs")
                    .class(tone),
                None => button(choice.name()).ghost().class("text-xs").class(tone),
            });
        }
        row
    }

    fn labels((lw, rw): (f32, f32)) -> NodeBuilder {
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .min_size(0.0, 0.0)
                    .padding_x(ROW_INSET)
                    .gap(0.0),
            )
            .child(
                label("Before")
                    .layout(
                        LayoutStyle::block()
                            .grow(lw)
                            .flex_basis(Dimension::Px(0.0))
                            .min_size(0.0, 0.0),
                    )
                    .class("text-xs")
                    .class(Token::Gray10.text()),
            )
            .child(
                label("After")
                    .layout(
                        LayoutStyle::block()
                            .grow(rw)
                            .flex_basis(Dimension::Px(0.0))
                            .min_size(0.0, 0.0),
                    )
                    .class("text-xs")
                    .class(Token::Gray10.text()),
            )
    }

    fn body(&self) -> Vec<NodeBuilder> {
        let pairs = pairs(&self.patch);
        let split = self.layout == DiffLayout::Split;
        let weights = weights(&pairs);
        let mut rows: Vec<NodeBuilder> = Vec::new();
        let (mut old, mut new, mut index) = (0, 0, 0);
        while let Some(pair) = pairs.get(index) {
            if rows.len() >= self.max_rows {
                rows.push(overflow_row());
                break;
            }
            if !split && pair.kind == Kind::Change {
                let count = change_run(&pairs, index);
                let group = pairs.get(index..index + count).unwrap_or_default();
                rows.extend(unified_rows(group, old, new));
                old += group.iter().filter(|item| item.left.is_some()).count();
                new += group.iter().filter(|item| item.right.is_some()).count();
                index += count;
                continue;
            }
            match pair.kind {
                Kind::Hunk => {
                    if let Some((from, to)) = pair.left.as_deref().and_then(starts) {
                        old = from;
                        new = to;
                    }
                    rows.push(text_row(
                        pair.left.clone().unwrap_or_default(),
                        "text-xs font-mono",
                        Token::Info11,
                    ));
                }
                Kind::Context | Kind::Change => {
                    rows.push(if split {
                        split_row(pair, old, new, weights)
                    } else {
                        source_row(pair.right.as_deref(), new, ' ', None)
                    });
                    old += usize::from(pair.left.is_some());
                    new += usize::from(pair.right.is_some());
                }
                Kind::Meta => rows.extend(
                    pair.left
                        .as_deref()
                        .filter(|line| !line.is_empty())
                        .map(|body| text_row(body.to_string(), "text-xs font-mono", Token::Gray10)),
                ),
            }
            index += 1;
        }
        rows
    }

    fn into_node_builder(self) -> NodeBuilder {
        let pairs = pairs(&self.patch);
        let added = pairs
            .iter()
            .filter(|pair| pair.kind == Kind::Change && pair.right.is_some())
            .count();
        let removed = pairs
            .iter()
            .filter(|pair| pair.kind == Kind::Change && pair.left.is_some())
            .count();
        let mut root = container()
            .role(Role::List)
            .label(self.path.clone().unwrap_or_else(|| String::from("Diff")))
            .description(format!("{added} added, {removed} removed"))
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .min_size(0.0, 0.0)
                    .overflow_x(Overflow::Hidden)
                    .gap(0.0),
            )
            .class("rounded")
            .class(Token::Field.bg())
            .style(
                Style::new()
                    .border_width(BORDER_WIDTH.0)
                    .border_theme(Token::Border.name()),
            )
            .child(self.header(added, removed));
        if self.layout == DiffLayout::Split {
            root = root.child(Self::labels(weights(&pairs)));
        }
        root = self.body().into_iter().fold(root, NodeBuilder::child);
        if let Some(status) = self.status.clone() {
            root = root.child(
                label(status)
                    .layout(
                        LayoutStyle::block()
                            .fill_width()
                            .padding_x(ROW_INSET)
                            .min_size(0.0, 0.0),
                    )
                    .class("text-xs")
                    .class(Token::Gray10.text()),
            );
        }
        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

impl StyleAccess for DiffBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for DiffBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(all(test, feature = "terminal"))]
#[path = "diff_tests.rs"]
mod tests;
