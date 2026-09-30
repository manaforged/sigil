use std::rc::Rc;

use crate::kernel::{Cx, IntoNode, NodeBuilder, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, signal};
use crate::style::{
    ClassSource, NodeStyleExt, Radius, StateHost, Style, StyleAccess, Theme, Token, space,
};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::super::shared::BuilderMutator;
use super::drag::DragState;
use super::list::build_tab_list;
use super::tab::line_strip_height;

pub(super) const TAB_HEIGHT: f32 = space(7);
pub(super) const STRIP_HEIGHT: f32 = space(9);

#[derive(Clone, Copy, Default)]
pub(super) enum StripChrome {
    #[default]
    Bare,
    Line,
    Pill,
}

pub(super) type CloseHandler = Rc<dyn Fn(usize) + 'static>;

pub struct TabsBuilder {
    active: Option<Signal<usize>>,
    labels: Vec<String>,
    disabled: Vec<usize>,
    aria_label: String,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
    contents: Vec<Option<Box<dyn IntoNode>>>,
    reorderable: bool,
    order: Option<Signal<Vec<usize>>>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    list_justify: Option<JustifyContent>,
    chrome: StripChrome,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn tabs() -> TabsBuilder {
    TabsBuilder {
        active: None,
        labels: Vec::new(),
        disabled: Vec::new(),
        aria_label: String::new(),
        mutators: Vec::new(),
        style_accumulator: Style::new(),
        contents: Vec::new(),
        reorderable: false,
        order: None,
        closeable: false,
        close_handler: None,
        list_justify: None,
        chrome: StripChrome::Bare,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl TabsBuilder {
    #[must_use]
    pub fn tab<F, N>(mut self, label: impl Into<String>, content: F) -> Self
    where
        F: FnOnce() -> N + 'static,
        N: IntoNode + 'static,
    {
        self.labels.push(label.into());
        self.contents.push(Some(Box::new(content)));
        self
    }

    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for label in labels {
            self.labels.push(label.into());
            self.contents.push(None);
        }
        self
    }

    #[must_use]
    pub const fn active(mut self, active: Signal<usize>) -> Self {
        self.active = Some(active);
        self
    }

    #[must_use]
    pub const fn line(mut self) -> Self {
        self.chrome = StripChrome::Line;
        self
    }

    #[must_use]
    pub const fn pill(mut self) -> Self {
        self.chrome = StripChrome::Pill;
        self
    }

    #[must_use]
    pub const fn justify_list(mut self, justify: JustifyContent) -> Self {
        self.list_justify = Some(justify);
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = label.into();
        self
    }

    #[must_use]
    pub fn disabled(mut self, indices: &[usize]) -> Self {
        self.disabled = indices.to_vec();
        self
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style_accumulator.merge(style);
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    #[must_use]
    pub const fn reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }

    #[must_use]
    pub const fn order(mut self, signal: Signal<Vec<usize>>) -> Self {
        self.order = Some(signal);
        self
    }

    #[must_use]
    pub const fn closeable(mut self, enabled: bool) -> Self {
        self.closeable = enabled;
        self
    }

    #[must_use]
    pub fn close(mut self, handler: impl Fn(usize) + 'static) -> Self {
        self.close_handler = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn into_container(self) -> NodeBuilder {
        container().child(self)
    }

    fn into_node_builder(mut self) -> NodeBuilder {
        let active = self.active.unwrap_or_else(|| signal(0));
        let order_signal = self.order;
        let drag = self.reorderable.then(DragState::new);
        let user_overlay = self.style_accumulator.take_state_overlay();
        let tab_list = apply_chrome(
            self.chrome,
            self.list_justify,
            build_tab_list(
                active,
                self.labels,
                &self.disabled,
                &self.aria_label,
                order_signal,
                drag,
                self.closeable,
                self.close_handler.as_ref().map(Rc::clone),
                user_overlay,
                self.chrome,
            ),
        );
        let mut root = if self.contents.iter().any(Option::is_some) {
            wrap_with_content(tab_list, active, self.contents)
        } else {
            tab_list
        };

        for mutate in self.mutators {
            root = mutate(root);
        }

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root.mutate(move |node| {
            let base = node
                .get_extension::<LayoutStyle>()
                .cloned()
                .unwrap_or_default();
            node.attach(self.layout_overrides.apply(base));
        })
    }
}

impl StyleAccess for TabsBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for TabsBuilder {}

impl WidgetLayoutExt for TabsBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for TabsBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn apply_chrome(
    chrome: StripChrome,
    justify: Option<JustifyContent>,
    strip: NodeBuilder,
) -> NodeBuilder {
    match chrome {
        StripChrome::Bare => match justify {
            None | Some(JustifyContent::Start) => strip,
            Some(justify) => container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .justify_content(justify),
                )
                .child(strip),
        },
        StripChrome::Line => wrap_line(strip, justify.unwrap_or(JustifyContent::Center)),
        StripChrome::Pill => wrap_pill(strip, justify.unwrap_or(JustifyContent::Center)),
    }
}

fn wrap_line(strip: NodeBuilder, justify: JustifyContent) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(line_strip_height())
                .shrink(0.0)
                .justify_content(justify)
                .align_items(CellForm::new(AlignItems::Center, AlignItems::Start).pick()),
        )
        .class("border-b")
        .class(Token::Border.border())
        .child(strip)
}

fn wrap_pill(strip: NodeBuilder, justify: JustifyContent) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .shrink(0.0)
                .padding_y(space(6))
                .justify_content(justify),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .height(STRIP_HEIGHT)
                        .padding(space(1))
                        .align_items(AlignItems::Center),
                )
                .class([Radius::Md.atom(), Token::SurfacePressed.bg()])
                .child(strip),
        )
}

use super::content::wrap_with_content;

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
