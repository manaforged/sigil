use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::animation_runtime::presets;
use crate::events::prelude::NodeEventsExt;
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TopLayer, container, text};
use crate::layout::ViewportOverlay;
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{IntervalId, Signal, cancel_interval, register_interval, signal};
use crate::style::{
    ClassSource, DisplayKind, NodeStyleExt, Size, Style, StyleAccess, StyleAtom, Theme,
    TransitionId,
};

use super::shared::BuilderMutator;

const BUBBLE_OFFSET_ABOVE: f32 = -32.0;
const BUBBLE_OFFSET_CLAMPED: f32 = 0.0;
const BUBBLE_HEIGHT: f32 = 28.0;
const BUBBLE_WIDTH: f32 = 160.0;
const BUBBLE_PADDING: f32 = 6.0;

const TOOLTIP_TRANSITION: TransitionId = presets::SMOOTH;

static BUBBLE_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent"),
    StyleAtom::TextThemeRef("on-accent"),
    StyleAtom::Transition(TOOLTIP_TRANSITION),
];

pub struct TooltipBuilder {
    label: String,
    shortcut: Option<String>,
    child: Option<Box<dyn IntoNode>>,
    clamp_top: bool,
    delay_ms: u64,
    mutators: Vec<BuilderMutator>,
    style_accumulator: Style,
}

#[must_use]
pub fn tooltip(label: impl Into<String>) -> TooltipBuilder {
    TooltipBuilder::new(label)
}

impl TooltipBuilder {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            child: None,
            clamp_top: false,
            delay_ms: 0,
            mutators: Vec::new(),
            style_accumulator: Style::new(),
        }
    }

    #[must_use]
    pub const fn delay(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    #[must_use]
    pub fn shortcut(mut self, shortcut: Option<impl Into<String>>) -> Self {
        self.shortcut = shortcut.map(Into::into);
        self
    }

    #[must_use]
    pub fn child(mut self, target: impl IntoNode + 'static) -> Self {
        self.child = Some(Box::new(target));
        self
    }

    #[must_use]
    pub const fn clamp_top(mut self) -> Self {
        self.clamp_top = true;
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
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let visible = signal(false);
        let bubble_top = if self.clamp_top {
            BUBBLE_OFFSET_CLAMPED
        } else {
            BUBBLE_OFFSET_ABOVE
        };

        let description = bubble_description(&self.label, self.shortcut.as_deref());
        let bubble = build_bubble(&self.label, self.shortcut.as_deref(), visible, bubble_top);

        let pending: Rc<Cell<Option<IntervalId>>> = Rc::new(Cell::new(None));
        let delay_ms = self.delay_ms;
        let pending_for_enter = Rc::clone(&pending);

        let mut root = wrapper_root(&description)
            .pointer_enter(move |event| {
                event.stop_propagation();
                if visible.peek() || pending_for_enter.get().is_some() {
                    return;
                }
                if delay_ms == 0 {
                    visible.set(true);
                    return;
                }
                let pending_for_cb = Rc::clone(&pending_for_enter);
                let id = register_interval(Duration::from_millis(delay_ms), move || {
                    visible.set(true);
                    if let Some(id) = pending_for_cb.take() {
                        cancel_interval(id);
                    }
                });
                pending_for_enter.set(Some(id));
            })
            .pointer_leave(move |event| {
                if let Some(id) = pending.take() {
                    cancel_interval(id);
                }
                if visible.peek() {
                    visible.set(false);
                }
                event.stop_propagation();
            });

        if let Some(child) = self.child {
            root = root.child(child);
        }
        root = root.child(bubble);

        for mutate in self.mutators {
            root = mutate(root);
        }

        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

impl StyleAccess for TooltipBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for TooltipBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn wrapper_root(description: &str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .description(description.to_owned())
        .layout(
            LayoutStyle::flex_column()
                .position(Position::Relative)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
}

fn bubble_description(label: &str, shortcut: Option<&str>) -> String {
    match shortcut {
        Some(keys) if !keys.is_empty() => format!("{label} ({keys})"),
        _ => label.to_owned(),
    }
}

fn build_bubble(
    label: &str,
    shortcut: Option<&str>,
    visible: Signal<bool>,
    top_px: f32,
) -> NodeBuilder {
    let aria_label = bubble_description(label, shortcut);
    let mut bubble = container()
        .role(Role::Tooltip)
        .label(aria_label)
        .with(TopLayer)
        .with(ViewportOverlay)
        .layout(bubble_layout(top_px, shortcut.is_some()))
        .class("rounded-sm")
        .class("shadow-md")
        .class(BUBBLE_BASE)
        .style_reactive(move || visibility_atoms(visible.get()));

    bubble = bubble.child(text(label.to_owned()).class("text-on-accent"));
    if let Some(keys) = shortcut.filter(|s| !s.is_empty()) {
        bubble = bubble.child(text(keys.to_owned()).class("text-on-accent"));
    }
    bubble
}

fn bubble_layout(top_px: f32, has_shortcut: bool) -> LayoutStyle {
    let theme = Theme::default();
    let gap = theme.gap(Size::Default).px_or_zero();
    let width = if has_shortcut {
        BUBBLE_WIDTH
    } else {
        BUBBLE_WIDTH * 0.75
    };
    LayoutStyle::flex_row()
        .fixed_size(width, BUBBLE_HEIGHT)
        .padding(BUBBLE_PADDING)
        .gap(gap)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .position(Position::Absolute)
        .inset_top(top_px)
        .inset_left(0.0)
}

const fn visibility_atoms(show: bool) -> &'static [StyleAtom] {
    if show {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

#[cfg(test)]
#[path = "tooltip_tests.rs"]
mod tests;
