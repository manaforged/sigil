use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::reactive::{Reactive, Signal, derived, signal};
use crate::style::{Radius, Style, Token, space};
use crate::widgets::Widget;

use super::super::layout_ext::LayoutOverrides;
use super::super::shared::caller_style;
use super::content::wrap_with_content;
use super::drag::TabDrag;
use super::list::{StripSpec, build_tab_list};
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

/// Builder of tabs.
pub struct TabsBuilder {
    active: Option<Signal<usize>>,
    labels: Vec<String>,
    label_source: Option<Box<dyn Fn() -> Vec<String>>>,
    disabled: Vec<usize>,
    aria_label: String,
    style_accumulator: Style,
    contents: Vec<Box<dyn IntoNode>>,
    reorderable: bool,
    order: Option<Signal<Vec<usize>>>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    list_justify: Option<JustifyContent>,
    chrome: StripChrome,
    layout_overrides: LayoutOverrides,
}

/// Creates a tabs widget.
#[must_use]
pub fn tabs() -> TabsBuilder {
    TabsBuilder {
        active: None,
        labels: Vec::new(),
        label_source: None,
        disabled: Vec::new(),
        aria_label: String::new(),
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
    /// Adds a tab.
    #[must_use]
    pub fn tab<F, N>(mut self, label: impl Into<String>, content: F) -> Self
    where
        F: FnOnce() -> N + 'static,
        N: IntoNode + 'static,
    {
        self.labels.push(label.into());
        self.contents.push(Box::new(content));
        self
    }

    /// Sets the labels.
    #[must_use]
    pub fn labels<S>(mut self, labels: impl Reactive<Vec<S>>) -> Self
    where
        S: Into<String> + 'static,
    {
        self.label_source = Some(Box::new(move || {
            labels.read().into_iter().map(Into::into).collect()
        }));
        self
    }

    /// Sets the active.
    #[must_use]
    pub const fn active(mut self, active: Signal<usize>) -> Self {
        self.active = Some(active);
        self
    }

    /// Applies the line option.
    #[must_use]
    pub const fn line(mut self) -> Self {
        self.chrome = StripChrome::Line;
        self
    }

    /// Applies the pill option.
    #[must_use]
    pub const fn pill(mut self) -> Self {
        self.chrome = StripChrome::Pill;
        self
    }

    /// Sets the justify list.
    #[must_use]
    pub const fn justify_list(mut self, justify: JustifyContent) -> Self {
        self.list_justify = Some(justify);
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, indices: &[usize]) -> Self {
        self.disabled = indices.to_vec();
        self
    }

    /// Sets the reorderable.
    #[must_use]
    pub const fn reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }

    /// Sets the order.
    #[must_use]
    pub const fn order(mut self, signal: Signal<Vec<usize>>) -> Self {
        self.order = Some(signal);
        self
    }

    /// Sets the closeable.
    #[must_use]
    pub const fn closeable(mut self, enabled: bool) -> Self {
        self.closeable = enabled;
        self
    }

    /// Sets the callback that closes a tab.
    #[must_use]
    pub fn close(mut self, handler: impl Fn(usize) + 'static) -> Self {
        self.close_handler = Some(Rc::new(handler));
        self
    }

    /// Converts this tabs builder to container.
    #[must_use]
    pub fn into_container(self) -> NodeBuilder {
        container().child(self)
    }

    fn root(mut self) -> NodeBuilder {
        let active = self.active.unwrap_or_else(|| signal(0));
        let labels = label_signal(self.labels, self.label_source.take());
        let spec = StripSpec {
            active,
            disabled: Rc::new(self.disabled),
            order: self.order,
            drag: self.reorderable.then(TabDrag::new),
            closeable: self.closeable,
            close_handler: self.close_handler,
            user_overlay: self.style_accumulator.take_state_overlay(),
            chrome: self.chrome,
        };
        let tab_list = apply_chrome(
            self.chrome,
            self.list_justify,
            build_tab_list(labels, &self.aria_label, spec),
        );
        let root = if self.contents.is_empty() {
            tab_list
        } else {
            wrap_with_content(tab_list, active, self.contents)
        };
        caller_style(root, self.style_accumulator)
    }
}

builder_impls!([] TabsBuilder: paint(style_accumulator), widget, states(style_accumulator), node);

fn label_signal(
    fixed: Vec<String>,
    source: Option<Box<dyn Fn() -> Vec<String>>>,
) -> Signal<Vec<String>> {
    match source {
        None => signal(fixed),
        Some(source) => derived(move || {
            let mut all = fixed.clone();
            all.extend(source());
            all
        }),
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
                        .shrink(0.0)
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
                .padding_y(Space::Xl)
                .justify_content(justify),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .height(STRIP_HEIGHT)
                        .padding(Space::Xs)
                        .align_items(AlignItems::Center),
                )
                .class([Radius::Md.atom(), Token::SurfacePressed.bg()])
                .child(strip),
        )
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
