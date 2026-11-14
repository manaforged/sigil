use crate::layout::Space;
use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, container};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::reactive::{Reactive, Signal, memo, signal};
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
#[must_use]
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

impl core::fmt::Debug for TabsBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TabsBuilder")
            .field("active", &self.active)
            .field("labels", &self.labels)
            .field("disabled", &self.disabled)
            .field("aria_label", &self.aria_label)
            .field("style_accumulator", &self.style_accumulator)
            .field("reorderable", &self.reorderable)
            .field("order", &self.order)
            .field("closeable", &self.closeable)
            .field("list_justify", &self.list_justify)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A tab strip with optional content panels; add tabs with `.tab()` or `.labels()`.
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
    /// Adds a tab with `label` and the panel that `content` builds.
    pub fn tab<F, N>(mut self, label: impl Into<String>, content: F) -> Self
    where
        F: FnOnce() -> N + 'static,
        N: IntoNode + 'static,
    {
        self.labels.push(label.into());
        self.contents.push(Box::new(content));
        self
    }

    /// Adds tabs with no panel from `labels`, after the tabs added with `.tab()`.
    pub fn labels<S>(mut self, labels: impl Reactive<Vec<S>>) -> Self
    where
        S: Into<String> + 'static,
    {
        self.label_source = Some(Box::new(move || {
            labels.read().into_iter().map(Into::into).collect()
        }));
        self
    }

    /// Binds the index of the active tab to `active`; the default is a new signal at 0.
    pub const fn active(mut self, active: Signal<usize>) -> Self {
        self.active = Some(active);
        self
    }

    /// Draws the tabs over a full-width bottom rule; the strip is centered by default.
    pub const fn line(mut self) -> Self {
        self.chrome = StripChrome::Line;
        self
    }

    /// Draws the tabs inside a rounded filled strip; the strip is centered by default.
    pub const fn pill(mut self) -> Self {
        self.chrome = StripChrome::Pill;
        self
    }

    /// Sets where the tab strip sits along the row.
    pub const fn justify_list(mut self, justify: JustifyContent) -> Self {
        self.list_justify = Some(justify);
        self
    }

    /// Sets the accessible name of the tab list.
    pub fn accessible(mut self, label: impl Into<String>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Disables the tabs at `indices`.
    pub fn disabled(mut self, indices: &[usize]) -> Self {
        self.disabled = indices.to_vec();
        self
    }

    /// Lets the user drag tabs to reorder them when `enabled` is true; needs `.order()`.
    pub const fn reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }

    /// Binds the display order of the tabs, as a list of tab indexes, to `signal`.
    pub const fn order(mut self, signal: Signal<Vec<usize>>) -> Self {
        self.order = Some(signal);
        self
    }

    /// Shows a close button on each tab when `enabled` is true.
    pub const fn closeable(mut self, enabled: bool) -> Self {
        self.closeable = enabled;
        self
    }

    /// Runs `handler` with the tab index when the user activates a close button.
    pub fn close(mut self, handler: impl Fn(usize) + 'static) -> Self {
        self.close_handler = Some(Rc::new(handler));
        self
    }

    /// Wraps the tabs in a plain container node.
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
        Some(source) => memo(move || {
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
                .class([Radius::Md.atom(), Token::SurfacePressed.background()])
                .child(strip),
        )
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
