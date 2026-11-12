use crate::animation::Property;
use crate::events::builder_ext as on;
use crate::events::{
    FileDropEvent, GestureEvent, ImeEvent, KeyEvent, PointerEvent, ScrollEvent, Shortcut,
    event_methods, handler_methods,
};
use crate::kernel::NodeBuilder;
use crate::layout::audit::{AllowedFlaws, Flaw};
use crate::layout::builder_ext::{asymmetric, layout_methods};
use crate::layout::reactive_position;
use crate::layout::{LayoutStyle, Length};
use crate::reactive::{Reactive, Signal};
use crate::style::builder_ext as look;
use crate::style::{FocusRing, StateStyleBuilder, Theme, Tone};
use crate::widgets::drag_drop::{self, DragState};

mod edit;

pub(crate) use edit::{Edit, LayoutEdit, NodeEdit, Sealed, StatePaint, StateTarget};

kittest_pub! {
    /// Merges a raw layout style into a builder.
    #[doc(hidden)]
    pub trait Layout: Sealed {
        /// Merges `style` into the node's layout.
        #[must_use]
        fn layout(self, style: LayoutStyle) -> Self {
            self.edit(NodeEdit(Edit::Layout(LayoutEdit::Merge(style))))
        }
    }
}

impl<T: Sealed> Layout for T {}

#[doc = include_str!("../../docs/widget.md")]
pub trait Widget: Sealed {
    /// Applies `f` to the root node that this builder produces.
    #[must_use]
    fn map_node(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self;

    /// Adds the styles that the Tailwind classes in `classes` name.
    #[must_use]
    fn class(self, classes: &'static str) -> Self {
        self.map_target(move |node| node.class(classes))
    }

    /// Sets the theme for the node and its children.
    #[must_use]
    fn theme(self, theme: Theme) -> Self {
        self.map_target(move |node| look::theme(node, theme))
    }

    /// Sets the tone that token colors resolve against in the node and its children.
    #[must_use]
    fn tone_scope(self, tone: Tone) -> Self {
        self.map_target(move |node| look::tone(node, tone))
    }

    /// Sets the focus ring: a `Signal<bool>` that shows it, or a `FocusRing` with color, width, offset and opacity.
    #[must_use]
    fn focus_ring(self, ring: impl Into<FocusRing>) -> Self {
        let ring = ring.into();
        self.map_target(move |node| look::focus_ring(node, ring))
    }

    /// Fades the node in and out over `ms` as `shown` changes.
    #[must_use]
    fn presence_tween(
        self,
        shown: Signal<bool>,
        ms: u16,
        easing: crate::animation::Easing,
    ) -> Self {
        self.map_target(move |node| {
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
        self.map_target(move |node| look::drive(node, property, value))
    }

    /// Hides the node while `shown` is false.
    #[must_use]
    fn visible(self, shown: impl Reactive<bool>) -> Self {
        self.map_target(move |node| look::visible(node, shown))
    }

    /// Applies the style built by `paint` while `active` is true.
    #[must_use]
    fn paint_when<F>(self, active: impl Reactive<bool>, paint: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = paint(StateStyleBuilder::new());
        self.map_target(move |node| look::paint_when(node, active, built))
    }

    /// Marks the node as intentionally asymmetric, with the reason.
    #[must_use]
    fn asymmetric(self, reason: &'static str) -> Self {
        self.map_node(move |node| asymmetric(node, reason))
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
        self.edit(NodeEdit(Edit::Layout(LayoutEdit::Width(width.into()))))
    }

    /// Sets the node's height.
    #[must_use]
    fn height(self, height: impl Into<Length>) -> Self {
        self.edit(NodeEdit(Edit::Layout(LayoutEdit::Height(height.into()))))
    }

    /// Sets the node's flex grow factor.
    #[must_use]
    fn grow(self, factor: f32) -> Self {
        self.edit(NodeEdit(Edit::Layout(LayoutEdit::Grow(factor))))
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
    fn selectable_text(self, yes: bool) -> Self {
        self.map_node(move |node| on::selectable_text(node, yes))
    }

    /// Focuses the node when it is added.
    #[must_use]
    fn autofocus(self) -> Self {
        self.edit(NodeEdit(Edit::Autofocus))
    }

    /// Focuses the node each time `trigger` changes.
    #[must_use]
    fn focus_when(self, trigger: Signal<u64>) -> Self {
        self.edit(NodeEdit(Edit::FocusWhen(trigger)))
    }
}

impl Sealed for NodeBuilder {
    fn edit(self, NodeEdit(edit): NodeEdit) -> Self {
        edit.apply(self)
    }

    fn map_target(self, f: impl FnOnce(Self) -> Self + 'static) -> Self {
        f(self)
    }
}

impl StateTarget for NodeBuilder {
    fn set_state(mut self, state: StatePaint) -> Self {
        self.pending_style().set_state(state.slot, state.atoms);
        self
    }
}

impl Widget for NodeBuilder {
    fn map_node(self, f: impl FnOnce(Self) -> Self + 'static) -> Self {
        f(self)
    }
}
