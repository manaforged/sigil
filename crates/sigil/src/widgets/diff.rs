use std::rc::Rc;

use crate::kernel::{AccessibleName, Cx, IntoNode, NodeBuilder, Role, container, node};
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, Overflow};
use crate::reactive::{Reactive, derived};
use crate::style::{BORDER_WIDTH, Paint, PaintTarget, Style, StyleSlot, Token, space};

use super::button::{ButtonBuilder, button};
use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::text;

mod patch;
mod rows;

use patch::{Kind, Pair, pairs, starts};
use rows::{source_row, split_row, unified_rows, weights};

const GUTTER_W: f32 = 56.0;
pub(super) const ROW_INSET: f32 = space(2);

const CHOICE_FONT_PX: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffView {
    Split,
    Unified,
}

impl DiffView {
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
            text(body)
                .nowrap()
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

type ViewChange = Rc<dyn Fn(DiffView)>;

#[derive(Clone)]
struct DiffShape {
    path: Option<String>,
    status: Option<String>,
    view: DiffView,
    max_rows: usize,
    on_change: Option<ViewChange>,
}

pub struct DiffBuilder {
    patch: Rc<dyn Reactive<String>>,
    shape: DiffShape,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn diff(patch: impl Reactive<String>) -> DiffBuilder {
    DiffBuilder {
        patch: Rc::new(patch),
        shape: DiffShape {
            path: None,
            status: None,
            view: DiffView::Split,
            max_rows: usize::MAX,
            on_change: None,
        },
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl DiffBuilder {
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.shape.path = Some(path.into());
        self
    }

    #[must_use]
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.shape.status = Some(status.into());
        self
    }

    #[must_use]
    pub const fn view(mut self, view: DiffView) -> Self {
        self.shape.view = view;
        self
    }

    #[must_use]
    pub const fn max_rows(mut self, rows: usize) -> Self {
        self.shape.max_rows = rows;
        self
    }

    #[must_use]
    pub fn change(mut self, on_change: impl Fn(DiffView) + 'static) -> Self {
        self.shape.on_change = Some(Rc::new(on_change));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let source = self.patch;
        let patch = derived(move || source.read());
        let summary = derived(move || {
            let (added, removed) = patch.with(|body| counts(&pairs(body)));
            format!("{added} added, {removed} removed")
        });
        let shape = self.shape;
        let mut root = container()
            .role(Role::List)
            .label(shape.path.clone().unwrap_or_else(|| String::from("Diff")))
            .mutate(move |target| target.a11y.description = Some(AccessibleName::Derived(summary)))
            .layout(self.layout_overrides.clone().apply(root_layout()))
            .class("rounded")
            .class(Token::Field.bg())
            .border_w(BORDER_WIDTH.0)
            .border_color(Token::Border)
            .children_reactive(move || patch.with(|body| shape.rows(body)));
        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

fn counts(pairs: &[Pair]) -> (usize, usize) {
    let changed = |side: fn(&Pair) -> bool| {
        pairs
            .iter()
            .filter(|pair| pair.kind == Kind::Change && side(pair))
            .count()
    };
    (
        changed(|pair| pair.right.is_some()),
        changed(|pair| pair.left.is_some()),
    )
}

impl DiffShape {
    fn rows(&self, patch: &str) -> Vec<Box<dyn IntoNode>> {
        let pairs = pairs(patch);
        let (added, removed) = counts(&pairs);
        let mut rows = vec![node(self.header(added, removed))];
        if self.view == DiffView::Split {
            rows.push(node(Self::labels(weights(&pairs))));
        }
        rows.extend(self.body(&pairs).into_iter().map(node));
        if let Some(status) = self.status.clone() {
            rows.push(node(
                text(status)
                    .nowrap()
                    .layout(
                        LayoutStyle::block()
                            .fill_width()
                            .padding_x(ROW_INSET)
                            .min_size(0.0, 0.0),
                    )
                    .class("text-xs")
                    .class(Token::Gray10.text()),
            ));
        }
        rows
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
                text(self.path.clone().unwrap_or_default())
                    .nowrap()
                    .ellipsize()
                    .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
                    .class("text-xs font-mono font-bold")
                    .class(Token::Gray12.text()),
            )
            .child(
                text(format!("+{added}"))
                    .nowrap()
                    .class("text-xs font-mono")
                    .class(Token::Success11.text()),
            )
            .child(
                text(format!("−{removed}"))
                    .nowrap()
                    .class("text-xs font-mono")
                    .class(Token::Error11.text()),
            );
        for choice in [DiffView::Split, DiffView::Unified] {
            let active = choice == self.view;
            let choice_button = paint_choice(button(choice.name()).ghost(), active);
            row = row.child(match self.on_change.clone() {
                Some(on_change) => choice_button.activate(move || on_change(choice)),
                None => choice_button,
            });
        }
        row
    }

    fn labels((lw, rw): (f32, f32)) -> NodeBuilder {
        let side = |text: &'static str, grow: f32, ruled: bool| {
            let half = container()
                .layout(
                    LayoutStyle::flex_row()
                        .grow(grow)
                        .flex_basis(Dimension::Px(0.0))
                        .padding_x(ROW_INSET)
                        .min_size(0.0, 0.0),
                )
                .child(
                    crate::kernel::text(text)
                        .nowrap()
                        .class("text-xs")
                        .class(Token::Gray10.text()),
                );
            if ruled {
                half.border_right(BORDER_WIDTH.0)
                    .border_color(Token::Border)
            } else {
                half
            }
        };
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .min_size(0.0, 0.0)
                    .gap(0.0),
            )
            .child(side("Before", lw, true))
            .child(side("After", rw, false))
    }

    fn body(&self, pairs: &[Pair]) -> Vec<NodeBuilder> {
        let split = self.view == DiffView::Split;
        let weights = weights(pairs);
        let mut rows: Vec<NodeBuilder> = Vec::new();
        let (mut old, mut new, mut index) = (0, 0, 0);
        while let Some(pair) = pairs.get(index) {
            if rows.len() >= self.max_rows {
                rows.push(overflow_row());
                break;
            }
            if !split && pair.kind == Kind::Change {
                let count = change_run(pairs, index);
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
}

impl PaintTarget for DiffBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for DiffBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

fn root_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .fill_width()
        .min_size(0.0, 0.0)
        .overflow_x(Overflow::Hidden)
        .gap(0.0)
}

impl IntoNode for DiffBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

#[cfg(all(test, feature = "terminal"))]
#[path = "diff_tests.rs"]
mod tests;

fn paint_choice(choice: ButtonBuilder, active: bool) -> ButtonBuilder {
    let choice = choice.font_size(CHOICE_FONT_PX);
    if active {
        choice.text_color(Token::Gray12).bg(Token::Gray4)
    } else {
        choice.text_color(Token::Gray10)
    }
}
