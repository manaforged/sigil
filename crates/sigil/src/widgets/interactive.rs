//! `interactive()` — opt-in interaction wrapper for plain containers.
//!
//! 5/5 Wave 1 (T2). A bare `container()` never becomes interactive by
//! accident: hittability, hover, and focus change occlusion and tab
//! order, so they arrive only through this wrapper. It gives the node a
//! deliberate, NON-propagating hover channel — ancestor hover survives,
//! unlike `InteractionState::pointer_enter`'s `stop_propagation`, which
//! is widget semantics and wrong for containers — optional focusability,
//! and it drives the `.hover()` / `.focus()` / `.pressed()` /
//! `.disabled()` fluent closures to actual paint through the DS2 overlay
//! machinery.
//!
//! ```rust,no_run
//! use sigil::prelude::*;
//!
//! interactive(container().class("p-4 rounded-md bg-surface"))
//!     .hover(|s| s.bg("accent-9"))
//!     .pressed(|s| s.bg("accent-11"));
//! ```

use std::cell::Cell;
use std::rc::Rc;

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::NodeBuilder;
use sigil_reactive::{Signal, effect, signal};
use sigil_style::{ClassSource, NodeStyleExt, ReactiveStateSlot, StateSlot, Style, StyleAccess};

/// Interactive wrapper around any [`NodeBuilder`]. Construct via
/// [`interactive()`]; chain `.focusable()` / `.disabled_signal()` and the
/// `Styled` state closures, then build like any widget.
pub struct InteractiveBuilder {
    inner: NodeBuilder,
    style: Style,
    disabled: Option<Signal<bool>>,
    focusable: bool,
}

/// Opt a node into deliberate interaction semantics: non-propagating
/// hover, pressed tracking, optional focus, and overlay-driven state
/// styling.
///
/// Event propagation is NOT stopped — ancestors keep their own
/// hover unless your handlers suppress it.
#[must_use]
pub fn interactive(inner: NodeBuilder) -> InteractiveBuilder {
    InteractiveBuilder {
        inner,
        style: Style::new(),
        disabled: None,
        focusable: false,
    }
}

impl InteractiveBuilder {
    /// Make the node focusable (Tab traversal + focus handlers). Off by
    /// default so styled containers never pollute tab order.
    #[must_use]
    pub const fn focusable(mut self, yes: bool) -> Self {
        self.focusable = yes;
        self
    }

    /// Drive the Disabled slot from a signal. While the signal is true,
    /// hover/press handlers go inert and the resolver applies the
    /// `.disabled_style(|s| …)` overlay slice.
    #[must_use]
    pub const fn disabled_signal(mut self, signal: Signal<bool>) -> Self {
        self.disabled = Some(signal);
        self
    }

    /// Attach utility classes, including `hover:` / `focus:` / `active:` /
    /// `disabled:` state prefixes. Base tokens style the rest state; the
    /// prefixed tokens feed this wrapper's overlay slots, which its
    /// interaction channel already drives — so a class string is a full
    /// alternative to the `.hover(|s| …)` / `.pressed(|s| …)` closures.
    ///
    /// ```rust,no_run
    /// use sigil::prelude::*;
    ///
    /// interactive(container())
    ///     .class("p-4 rounded-md bg-surface hover:bg-accent-9 active:bg-accent-11");
    /// ```
    #[must_use]
    pub fn class(mut self, classes: &str) -> Self {
        self.style = core::mem::take(&mut self.style).class(classes);
        self
    }

    /// Consume into a [`NodeBuilder`] with the interaction wiring
    /// installed: signals → slot cell → resolver overlay application.
    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let mut style = self.style;
        let overlay = style.take_state_overlay();
        let base_atoms = style.into_atoms();

        let hovered = signal(false);
        let pressed = signal(false);
        let focused = signal(false);
        let disabled = self.disabled;

        let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
        publish_slot_on_change(&slot_cell, hovered, pressed, focused, disabled);

        let mut node = self.inner.focusable(self.focusable);
        if !base_atoms.is_empty() {
            node = node.class(ClassSource::Owned(Box::new(base_atoms)));
        }
        if !overlay.is_empty() {
            node = node.state_overlay(overlay, slot_cell);
        }
        node.pointer_enter(move |_event| {
            if !disabled_is(disabled.as_ref()) {
                set_if_changed(hovered, true);
            }
        })
        .pointer_leave(move |_event| {
            set_if_changed(hovered, false);
            set_if_changed(pressed, false);
        })
        .pointer_down(move |_event| {
            if !disabled_is(disabled.as_ref()) {
                set_if_changed(pressed, true);
            }
        })
        .pointer_up(move |_event| {
            set_if_changed(pressed, false);
        })
        .focus_in(move || {
            if !disabled_is(disabled.as_ref()) {
                set_if_changed(focused, true);
            }
        })
        .focus_out(move || {
            set_if_changed(focused, false);
        })
    }
}

impl StyleAccess for InteractiveBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl sigil_style::StateHost for InteractiveBuilder {}

impl sigil_kernel::IntoNode for InteractiveBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

/// Publish the current [`StateSlot`] whenever any interaction signal
/// changes. Precedence mirrors the widget variants: Disabled wins, then
/// Pressed, Hovered, Focused, Rest.
fn publish_slot_on_change(
    slot_cell: &ReactiveStateSlot,
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    focused: Signal<bool>,
    disabled: Option<Signal<bool>>,
) {
    let cell = Rc::clone(slot_cell);
    effect(move || {
        let slot = if disabled_is(disabled.as_ref()) {
            StateSlot::Disabled
        } else if pressed.get() {
            StateSlot::Pressed
        } else if hovered.get() {
            StateSlot::Hovered
        } else if focused.get() {
            StateSlot::Focused
        } else {
            StateSlot::Rest
        };
        cell.set(slot);
    });
}

fn disabled_is(disabled: Option<&Signal<bool>>) -> bool {
    disabled.copied().is_some_and(Signal::get)
}

fn set_if_changed(signal: Signal<bool>, next: bool) {
    if signal.peek() != next {
        signal.set(next);
    }
}
