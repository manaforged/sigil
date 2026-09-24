use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::animation_runtime::presets;
use crate::events::focus_from_keyboard;
use crate::events::mutate_handlers;
use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TopLayer, container, text};
use crate::layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::layout::{Popup, PopupSide, ViewportOverlay};
use crate::reactive::{IntervalId, Signal, cancel_interval, register_interval, signal};
use crate::style::{
    ClassSource, DisplayKind, Metrics, NodeStyleExt, Size, Style, StyleAccess, StyleAtom, Theme,
    Token, TransitionId, space,
};

use super::popup::{POPUP_GAP, surface};
use super::shared::BuilderMutator;

const BUBBLE_MAX_WIDTH: f32 = space(60);

const TOOLTIP_TRANSITION: TransitionId = presets::SMOOTH;

static BUBBLE_BASE: &[StyleAtom] = &[StyleAtom::Transition(TOOLTIP_TRANSITION)];

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
        let side = if self.clamp_top {
            PopupSide::Below
        } else {
            PopupSide::Above
        };

        let description = bubble_description(&self.label, self.shortcut.as_deref());
        let bubble = build_bubble(&self.label, self.shortcut.as_deref(), visible, side);

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

        if let Some(inner) = self.child {
            root = root.child(FocusTarget { inner, visible });
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

struct FocusTarget {
    inner: Box<dyn IntoNode>,
    visible: Signal<bool>,
}

impl IntoNode for FocusTarget {
    fn build(self, cx: &mut Cx) -> NodeId {
        let id = self.inner.build(cx);
        if let Some(target) = first_focusable(cx, id) {
            watch_focus(cx, target, self.visible);
        }
        id
    }
}

fn first_focusable(cx: &Cx, id: NodeId) -> Option<NodeId> {
    let node = cx.get(id)?;
    if node.a11y.focusable {
        return Some(id);
    }
    node.children()
        .iter()
        .find_map(|child| first_focusable(cx, *child))
}

fn watch_focus(cx: &mut Cx, target: NodeId, visible: Signal<bool>) {
    let Some(node) = cx.get_mut(target) else {
        return;
    };
    mutate_handlers(node, move |handlers| {
        handlers.add_focus_in(Box::new(move || {
            if focus_from_keyboard() {
                visible.set(true);
            }
        }));
        handlers.add_focus_out(Box::new(move || visible.set(false)));
        handlers.add_key(
            Key::Escape.into(),
            Box::new(move |event| {
                if visible.peek() {
                    visible.set(false);
                    event.stop_propagation();
                }
            }),
        );
    });
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
    side: PopupSide,
) -> NodeBuilder {
    let aria_label = bubble_description(label, shortcut);
    let mut bubble = surface(Role::Tooltip)
        .label(aria_label)
        .with(TopLayer)
        .with(ViewportOverlay)
        .with(Popup {
            side,
            center: true,
            sibling: true,
            gap: POPUP_GAP,
        })
        .layout(bubble_layout())
        .class(BUBBLE_BASE)
        .style_reactive(move || visibility_atoms(visible.get()));

    bubble = bubble.child(
        text(label.to_owned())
            .class("text-sm")
            .class(Token::OnSurface.text()),
    );
    if let Some(keys) = shortcut.filter(|s| !s.is_empty()) {
        bubble = bubble.child(
            text(keys.to_owned())
                .class("text-xs")
                .class(Token::OnSurfaceMuted.text()),
        );
    }
    bubble
}

fn bubble_layout() -> LayoutStyle {
    let metrics = Metrics::current();
    LayoutStyle::flex_column()
        .max_width(BUBBLE_MAX_WIDTH)
        .padding_x(metrics.padding(Size::Compact))
        .padding_y(metrics.overlay_padding())
        .gap(metrics.overlay_padding())
        .align_items(AlignItems::Start)
        .justify_content(JustifyContent::Center)
        .position(Position::Absolute)
        .inset_top(0.0)
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
