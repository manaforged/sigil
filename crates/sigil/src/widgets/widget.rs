use crate::animation::Property;
use crate::events::builder_ext as on;
use crate::events::{
    A11yEdit, DragPhase, FileDropEvent, GestureEvent, ImeEvent, KeyEvent, PointerEvent,
    ScrollEvent, Shortcut, event_methods, handler_methods,
};
use crate::kernel::NodeBuilder;
use crate::layout::audit::{AllowedFlaws, Flaw};
use crate::layout::builder_ext::{install_layout, layout_methods};
use crate::layout::reactive_position;
use crate::layout::{CaretGeometry, LayoutStyle, Length, TextStyleCapture};
use crate::reactive::{Reactive, Signal};
use crate::style::builder_ext as look;
use crate::style::{FocusRing, StateStyleBuilder, TextStyle, Theme, Tone};
use crate::widgets::drag_drop::{self, DragState};

mod edit;

pub(crate) use edit::LayoutEdit;

#[doc = include_str!("../../docs/widget.md")]
pub trait Widget: Sized {
    /// Applies `f` to the root node that this builder produces.
    #[must_use]
    fn map_node(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self;

    /// Merges `style` into the node's layout.
    #[must_use]
    fn layout(self, style: LayoutStyle) -> Self {
        self.map_node(move |node| node.mutate(move |n| install_layout(n, &style)))
    }

    /// Adds the styles that the Tailwind classes in `classes` name.
    #[must_use]
    fn class(self, classes: &'static str) -> Self {
        self.map_node(move |node| node.class(classes))
    }

    /// Sets the theme for the node and its children.
    #[must_use]
    fn theme(self, theme: Theme) -> Self {
        self.map_node(move |node| look::theme(node, theme))
    }

    /// Sets the tone that token colors resolve against in the node and its children.
    #[must_use]
    fn tone_scope(self, tone: Tone) -> Self {
        self.map_node(move |node| look::tone(node, tone))
    }

    /// Sets the focus ring: a `Signal<bool>` that shows it, or a `FocusRing` with color, width, offset and opacity.
    #[must_use]
    fn focus_ring(self, ring: impl Into<FocusRing>) -> Self {
        let ring = ring.into();
        self.map_node(move |node| look::focus_ring(node, ring))
    }

    /// Fades the node in and out over `ms` as `shown` changes.
    #[must_use]
    fn presence(self, shown: Signal<bool>, ms: u16, easing: crate::animation::Easing) -> Self {
        self.map_node(move |node| {
            look::presence(
                node,
                shown,
                crate::animation::runtime::AnimationConfig::Tween {
                    ms,
                    easing,
                    delay_ms: 0,
                },
            )
        })
    }

    /// Drives `property`, such as `Property::Width`, from `value` in the unit its variant documents.
    #[must_use]
    fn drive(self, property: fn(f32) -> Property, value: Signal<f32>) -> Self {
        self.map_node(move |node| look::drive(node, property, value))
    }

    /// Hides the node while `shown` is false.
    #[must_use]
    fn visible(self, shown: impl Reactive<bool>) -> Self {
        self.map_node(move |node| look::visible(node, shown))
    }

    /// Applies the style built by `paint` while `active` is true.
    #[must_use]
    fn paint_when<F>(self, active: impl Reactive<bool>, paint: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = paint(StateStyleBuilder::new());
        self.map_node(move |node| look::paint_when(node, active, built))
    }

    /// Opts the node and its children out of a design rule, with the reason.
    #[must_use]
    fn allow_flaw(self, kind: Flaw, _reason: &'static str) -> Self {
        self.map_node(move |node| {
            node.mutate(move |node| {
                if let Some(allowed) = node.extension_mut::<AllowedFlaws>() {
                    allowed.0.push(kind);
                } else {
                    let _ = node.attach(AllowedFlaws(smallvec::smallvec![kind]));
                }
            })
        })
    }

    /// Sets the node's width.
    #[must_use]
    fn width(self, width: impl Into<Length>) -> Self {
        let edit = LayoutEdit::Width(width.into());
        self.map_node(move |node| node.mutate(move |n| edit.install(n)))
    }

    /// Sets the node's height.
    #[must_use]
    fn height(self, height: impl Into<Length>) -> Self {
        let edit = LayoutEdit::Height(height.into());
        self.map_node(move |node| node.mutate(move |n| edit.install(n)))
    }

    /// Sets the node's flex grow factor.
    #[must_use]
    fn grow(self, factor: f32) -> Self {
        let edit = LayoutEdit::Grow(factor);
        self.map_node(move |node| node.mutate(move |n| edit.install(n)))
    }

    layout_methods!();

    /// Writes the node's laid-out position into `pos` after each layout.
    #[must_use]
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self {
        self.map_node(move |node| reactive_position::position(node, pos))
    }

    /// Writes the node's laid-out size into `size` after each layout.
    #[must_use]
    fn size_reactive(self, size: Signal<(f32, f32)>) -> Self {
        self.map_node(move |node| reactive_position::size(node, size))
    }

    /// Writes the node's laid-out bounds into `bounds` after each layout.
    #[must_use]
    fn bounds_reactive(self, bounds: Signal<(f32, f32, f32, f32)>) -> Self {
        self.map_node(move |node| reactive_position::bounds(node, bounds))
    }

    /// Makes the node a drag source that carries `payload`.
    #[must_use]
    fn drag_source<T: Clone + 'static>(self, state: DragState<T>, payload: T) -> Self {
        self.map_node(move |node| drag_drop::source(node, state, payload))
    }

    /// Makes the node a drop target that calls `on_drop` for payloads that `accepts` allows.
    #[must_use]
    fn drop_target<T: Clone + 'static>(
        self,
        state: DragState<T>,
        accepts: impl Fn(&T) -> bool + 'static,
        on_drop: impl Fn(T) + 'static,
    ) -> Self {
        self.map_node(move |node| drag_drop::target(node, state, accepts, on_drop))
    }

    handler_methods!(event_methods);

    /// Runs `h` when a pointer button goes up on the node, and marks the node clickable.
    #[must_use]
    fn pointer_up<H>(self, h: H) -> Self
    where
        H: Fn(&mut PointerEvent) + 'static,
    {
        self.map_node(move |node| on::pointer_up(node, h))
    }

    /// Enables the input method on the node while `f` returns true.
    #[must_use]
    fn ime_eligible<F>(self, f: F) -> Self
    where
        F: Fn() -> bool + 'static,
    {
        self.map_node(move |node| on::ime_eligible(node, f))
    }

    /// Lets the user select the node's text when `yes` is true.
    #[must_use]
    fn selectable(self, yes: bool) -> Self {
        self.map_node(move |node| on::selectable_text(node, yes))
    }

    /// Focuses the node when it is added.
    #[must_use]
    fn autofocus(self) -> Self {
        self.map_node(on::autofocus)
    }

    /// Focuses the node each time `trigger` changes.
    #[must_use]
    fn focus_when(self, trigger: Signal<u64>) -> Self {
        self.map_node(move |node| on::focus_when(node, trigger))
    }

    /// Mirrors pointer hover state into `out` when it changes.
    #[must_use]
    fn track_hovered(self, out: Signal<bool>) -> Self {
        self.map_node(move |node| on::track_hovered(node, out))
    }

    /// Mirrors focus state into `out` when it changes.
    #[must_use]
    fn track_focused(self, out: Signal<bool>) -> Self {
        self.map_node(move |node| on::track_focused(node, out))
    }

    /// Mirrors primary pointer press state into `out` when it changes, without marking clickable.
    #[must_use]
    fn track_pressed(self, out: Signal<bool>) -> Self {
        self.map_node(move |node| on::track_pressed(node, out))
    }

    /// Runs `handler` across a captured primary pointer drag lifecycle.
    #[must_use]
    fn pointer_drag(self, handler: impl FnMut(DragPhase, &mut PointerEvent) + 'static) -> Self {
        self.map_node(move |node| on::pointer_drag(node, handler))
    }

    /// Handles assistive-technology edit and value-adjustment requests on this node.
    #[must_use]
    fn a11y_edit(self, handler: impl Fn(&A11yEdit) -> bool + 'static) -> Self {
        self.map_node(move |node| on::a11y_edit(node, handler))
    }

    /// Records the node's resolved [`TextStyle`] into `out` after each layout pass.
    #[must_use]
    fn capture_text_style(self, out: Signal<TextStyle>) -> Self {
        self.map_node(move |node| {
            node.mutate(move |n| {
                let _ = n.attach(TextStyleCapture::new(out));
            })
        })
    }

    /// Supplies a reactive node-local caret rectangle for IME candidate window placement.
    #[must_use]
    fn ime_caret(self, caret: impl Reactive<Option<CaretGeometry>>) -> Self {
        let signal = caret.into_signal();
        self.map_node(move |node| on::ime_caret_signal(node, signal))
    }
}

impl Widget for NodeBuilder {
    fn map_node(self, f: impl FnOnce(Self) -> Self + 'static) -> Self {
        f(self)
    }
}
