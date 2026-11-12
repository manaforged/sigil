use crate::layout::Space;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::animation::runtime::presets;
use crate::events::Key;
use crate::events::focus_from_keyboard;
use crate::events::mutate_handlers;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Position};
use crate::layout::{Popup, PopupSide};
use crate::reactive::{IntervalId, Signal, interval, signal};
use crate::style::{ControlSize, Metrics, Style, StyleAtom, Token, space};
use crate::widgets::{Layout, Widget};

use super::layout_ext::LayoutOverrides;
use super::popup::{POPUP_GAP, PopupAnchor, PopupLayer, hide, surface};
use super::shared::caller_style;

const BUBBLE_MAX_WIDTH: f32 = space(60);

static BUBBLE_BASE: &[StyleAtom] = &[StyleAtom::Transition(presets::SMOOTH)];

/// Builder of a tooltip.
#[must_use]
pub struct TooltipBuilder {
    label: String,
    shortcut: Option<String>,
    child: Option<Box<dyn IntoNode>>,
    clamp_top: bool,
    delay_ms: u64,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

impl core::fmt::Debug for TooltipBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TooltipBuilder")
            .field("label", &self.label)
            .field("shortcut", &self.shortcut)
            .field("clamp_top", &self.clamp_top)
            .field("delay_ms", &self.delay_ms)
            .field("style_accumulator", &self.style_accumulator)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// A hover tooltip.
pub fn tooltip(label: impl Into<String>) -> TooltipBuilder {
    TooltipBuilder::new(label)
}

impl TooltipBuilder {
    /// Creates a tooltip builder.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            child: None,
            clamp_top: false,
            delay_ms: 0,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the delay.
    pub const fn delay(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    /// Sets the hint.
    pub fn hint(mut self, hint: Option<impl Into<String>>) -> Self {
        self.shortcut = hint.map(Into::into);
        self
    }

    /// Adds a child.
    pub fn child(mut self, target: impl IntoNode + 'static) -> Self {
        self.child = Some(Box::new(target));
        self
    }

    /// Applies the clamp top option.
    pub const fn clamp_top(mut self) -> Self {
        self.clamp_top = true;
        self
    }

    fn root(self) -> NodeBuilder {
        let visible = signal(false);
        let side = if self.clamp_top {
            PopupSide::Below
        } else {
            PopupSide::Above
        };
        let description = bubble_description(&self.label, self.shortcut.as_deref());
        let bubble = build_bubble(
            &self.label,
            self.shortcut.as_deref(),
            &description,
            (visible, side),
        );
        let root = container()
            .role(Role::GenericContainer)
            .description(description)
            .layout(
                LayoutStyle::flex_column()
                    .position(Position::Relative)
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            );
        let reveal = Reveal::new(visible);
        let mut root = hover_reveal(root, reveal.clone(), self.delay_ms);
        if let Some(inner) = self.child {
            root = root.child(TooltipTarget { inner, reveal });
        }
        root = root.child(bubble);
        caller_style(root, self.style_accumulator)
    }
}

builder_impls!([] TooltipBuilder: paint(style_accumulator), widget, node);

#[derive(Clone)]
struct Reveal {
    visible: Signal<bool>,
    pending: Rc<Cell<Option<IntervalId>>>,
    focused: Rc<Cell<bool>>,
}

impl Reveal {
    fn new(visible: Signal<bool>) -> Self {
        Self {
            visible,
            pending: Rc::new(Cell::new(None)),
            focused: Rc::new(Cell::new(false)),
        }
    }

    fn cancel_pending(&self) {
        if let Some(id) = self.pending.take() {
            id.cancel();
        }
    }

    fn hide(&self) -> bool {
        self.cancel_pending();
        hide(self.visible)
    }
}

fn hover_reveal(root: NodeBuilder, reveal: Reveal, delay_ms: u64) -> NodeBuilder {
    let enter = reveal.clone();
    root.pointer_enter(move |event| {
        event.stop_propagation();
        if enter.visible.peek() || enter.pending.get().is_some() {
            return;
        }
        if delay_ms == 0 {
            enter.visible.set(true);
            return;
        }
        let fire = enter.clone();
        let id = interval(Duration::from_millis(delay_ms), move || {
            fire.visible.set(true);
            fire.cancel_pending();
        });
        enter.pending.set(Some(id));
    })
    .pointer_leave(move |event| {
        if reveal.focused.get() {
            reveal.cancel_pending();
        } else {
            reveal.hide();
        }
        event.stop_propagation();
    })
}

struct TooltipTarget {
    inner: Box<dyn IntoNode>,
    reveal: Reveal,
}

impl IntoNode for TooltipTarget {
    fn build(self, cx: &mut Cx) -> NodeId {
        let id = self.inner.build(cx);
        if let Some(target) = first_focusable(cx, id) {
            watch_focus(cx, target, self.reveal);
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

fn watch_focus(cx: &mut Cx, target: NodeId, reveal: Reveal) {
    let Some(node) = cx.get_mut(target) else {
        return;
    };
    mutate_handlers(node, move |handlers| {
        let focus_in = reveal.clone();
        handlers.add_focus_in(Box::new(move || {
            if focus_from_keyboard() {
                focus_in.focused.set(true);
                focus_in.visible.set(true);
            }
        }));
        let focus_out = reveal.clone();
        handlers.add_focus_out(Box::new(move || {
            focus_out.focused.set(false);
            focus_out.hide();
        }));
        handlers.add_key(
            Key::Escape.into(),
            Box::new(move |event| {
                if reveal.hide() {
                    event.stop_propagation();
                }
            }),
        );
    });
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
    description: &str,
    (visible, side): (Signal<bool>, PopupSide),
) -> NodeBuilder {
    let metrics = Metrics::current();
    let anchor = PopupAnchor::Popup(Popup {
        side,
        center: true,
        sibling: true,
        gap: POPUP_GAP,
    });
    let mut bubble = surface(Role::Tooltip)
        .hittable(false)
        .label(description.to_owned())
        .layout(
            LayoutStyle::flex_column()
                .max_width(BUBBLE_MAX_WIDTH)
                .padding_x(Space::px(metrics.control_padding(ControlSize::Compact)))
                .padding_y(Space::px(metrics.overlay_padding()))
                .gap(Space::px(metrics.overlay_padding()))
                .align_items(AlignItems::Start)
                .justify_content(JustifyContent::Center)
                .position(Position::Absolute)
                .inset_top(0.0)
                .inset_left(0.0),
        )
        .class(BUBBLE_BASE)
        .child(
            text(label.to_owned())
                .hittable(false)
                .class("text-sm")
                .class(Token::OnSurface.text()),
        );
    if let Some(keys) = shortcut.filter(|s| !s.is_empty()) {
        bubble = bubble.child(
            text(keys.to_owned())
                .hittable(false)
                .class("text-xs")
                .class(Token::OnSurfaceMuted.text()),
        );
    }
    PopupLayer::new(visible, anchor).wrap(bubble)
}

#[cfg(test)]
#[path = "tooltip_tests.rs"]
mod tests;
