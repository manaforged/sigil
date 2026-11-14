use std::time::Duration;

use crate::animation::{Property, Transition};
use crate::events::builder_ext as on;
use crate::events::{
    A11yEdit, CursorIcon, DragPhase, DropEvent, FileDropEvent, GestureEvent, ImeEvent, KeyEvent,
    PointerEvent, ScrollEvent, Shortcut, ValueStep, event_methods, handler_methods,
};
use crate::geometry::{Point, Rect, Size};
use crate::kernel::{AccessibleName, NodeBuilder, Role};
use crate::layout::audit::{AllowedFlaws, Flaw};
use crate::layout::builder_ext::{install_layout, layout_methods};
use crate::layout::reactive_position;
use crate::layout::{AlignItems, CaretGeometry, LayoutStyle, Length, Position, TextStyleCapture};
use crate::reactive::{Reactive, Signal};
use crate::style::builder_ext as look;
use crate::style::presence::Edge;
use crate::style::{FocusRing, StateStyleBuilder, TextStyle, Theme, Tone};
use crate::widgets::Side;
use crate::widgets::drag_drop::{self, DragState};
use crate::widgets::popup::{float, placement};

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

    /// Restyles the colors and corner radii of the node and its children with `theme`; control sizes still come from the app theme.
    #[must_use]
    fn theme(self, theme: impl Reactive<Theme>) -> Self {
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

    /// Fades the node in when `shown` turns true and out when it turns false, over `ms` milliseconds with `easing`.
    #[must_use]
    fn presence(self, shown: Signal<bool>, ms: u16, easing: crate::animation::Easing) -> Self {
        let transition = Transition::opacity()
            .duration(Duration::from_millis(u64::from(ms)))
            .ease(easing);
        self.map_node(move |node| look::presence(node, shown, transition))
    }

    /// Starts the node from `property` when it mounts, then animates it to its own style with its `transition`.
    #[must_use]
    fn enter(self, property: Property) -> Self {
        self.map_node(move |node| look::motion(node, Edge::Enter, property))
    }

    /// Animates the node to `property` with its `transition` when `show`, `switch` or `for_each` removes it; the leaving node still takes its layout space.
    #[must_use]
    fn exit(self, property: Property) -> Self {
        self.map_node(move |node| look::motion(node, Edge::Exit, property))
    }

    /// Morphs the node from the last place and corners of the node with the same `key` that just left or is leaving, like Motion `layoutId`.
    #[must_use]
    fn shared(self, key: impl std::hash::Hash) -> Self {
        let key = crate::animation::runtime::shared::Shared::of(&key);
        self.map_node(move |node| crate::animation::runtime::shared::attach(node, key))
    }

    /// Drives `property`, such as `Property::Width`, from `value` in the unit its variant documents.
    #[must_use]
    fn drive(self, property: fn(f32) -> Property, value: impl Reactive<f32>) -> Self {
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

    /// Sets the node's share of its parent's free space along the main axis, relative to its siblings' factors; 0 does not grow.
    #[must_use]
    fn grow(self, factor: f32) -> Self {
        let edit = LayoutEdit::Grow(factor);
        self.map_node(move |node| node.mutate(move |n| edit.install(n)))
    }

    layout_methods!();

    /// Shifts the node's laid-out position by `offset` without re-laying out its siblings.
    #[must_use]
    fn offset(self, offset: impl Reactive<Point>) -> Self {
        let pos = offset.into_signal();
        self.map_node(move |node| reactive_position::offset(node, pos))
    }

    /// Writes the node's laid-out bounding rectangle into `out` after each layout pass.
    #[must_use]
    fn track_bounds(self, out: Signal<Rect>) -> Self {
        self.map_node(move |node| reactive_position::bounds(node, out))
    }

    /// Writes the node's laid-out origin into `out` after each layout pass.
    #[must_use]
    fn track_position(self, out: Signal<Point>) -> Self {
        self.map_node(move |node| reactive_position::position(node, out))
    }

    /// Writes the node's laid-out size into `out` after each layout pass.
    #[must_use]
    fn track_size(self, out: Signal<Size>) -> Self {
        self.map_node(move |node| reactive_position::size(node, out))
    }

    /// Writes the share of the node inside its clipping scroll views and the window, from 0.0 to 1.0, into `out` after each layout pass and scroll.
    #[must_use]
    fn track_visible(self, out: Signal<f32>) -> Self {
        self.map_node(move |node| reactive_position::visible(node, out))
    }

    /// Makes the node a drag source that carries `payload`.
    #[must_use]
    fn drag_source<T: Clone + 'static>(self, state: DragState<T>, payload: T) -> Self {
        self.map_node(move |node| drag_drop::source(node, state, payload))
    }

    /// Makes the node a drop target that calls `drop` for payloads that `accepts` allows.
    #[must_use]
    fn drop_target<T: Clone + 'static>(
        self,
        state: DragState<T>,
        accepts: impl Fn(&T) -> bool + 'static,
        drop: impl Fn(DropEvent<T>) + 'static,
    ) -> Self {
        self.map_node(move |node| drag_drop::target(node, state, accepts, drop))
    }

    /// Writes the target-local pointer position into `local_cursor` while an accepted drag hovers this node.
    #[must_use]
    fn drop_hover<T: Clone + 'static>(
        self,
        state: DragState<T>,
        local_cursor: Signal<Option<Point>>,
    ) -> Self {
        self.map_node(move |node| drag_drop::hover(node, &state, local_cursor))
    }

    handler_methods!(event_methods);

    /// Runs `handler` when `shortcut` is pressed anywhere in the window, and exposes it to assistive technology.
    #[must_use]
    fn shortcut(
        self,
        shortcut: impl Into<Shortcut>,
        handler: impl Fn(&mut KeyEvent) + 'static,
    ) -> Self {
        let shortcut = shortcut.into();
        self.map_node(move |node| on::shortcut(node, shortcut, handler))
    }

    /// Sets the shortcut that the node reports to assistive technology without registering a key handler.
    #[must_use]
    fn accessible_shortcut(self, shortcut: impl Into<Shortcut>) -> Self {
        let shortcut = shortcut.into();
        self.map_node(move |node| on::accessible_shortcut(node, shortcut))
    }

    /// Calls `on_step` with the step paired to a key in `keys` when that key's event reaches the node.
    #[must_use]
    fn step_keys<S, H>(self, keys: impl IntoIterator<Item = (S, ValueStep)>, on_step: H) -> Self
    where
        S: Into<Shortcut>,
        H: Fn(ValueStep, &mut KeyEvent) + Clone + 'static,
    {
        keys.into_iter().fold(self, |widget, (key, step)| {
            let on_step = on_step.clone();
            widget.key(key, move |event| on_step(step, event))
        })
    }

    /// Sets the name a screen reader announces for this widget.
    #[must_use]
    fn accessible(self, name: impl Into<AccessibleName>) -> Self {
        let name = name.into();
        self.map_node(move |node| node.accessible(name))
    }

    /// Sets the role that a screen reader announces for this widget.
    #[must_use]
    fn role(self, role: Role) -> Self {
        self.map_node(move |node| node.role(role))
    }

    /// Sets the description that a screen reader reads after the name.
    #[must_use]
    fn description(self, description: impl Into<String>) -> Self {
        let description = description.into();
        self.map_node(move |node| node.description(description))
    }

    /// Sets whether the widget and its children stay out of the accessibility tree, like `aria-hidden`.
    #[must_use]
    fn accessibility_hidden(self, yes: bool) -> Self {
        self.map_node(move |node| node.accessibility_hidden(yes))
    }

    /// Sets whether the widget can take keyboard focus.
    #[must_use]
    fn focusable(self, yes: bool) -> Self {
        self.map_node(move |node| node.focusable(yes))
    }

    /// Sets the tab order: lower indexes come first, and a negative index skips the widget.
    #[must_use]
    fn tab_index(self, index: i32) -> Self {
        self.map_node(move |node| node.tab_index(index))
    }

    /// Floats the widget above the tree on `side` of its first sibling, aligned by `align`, and flips or shifts it to stay in the window.
    #[must_use]
    fn anchor(self, side: Side, align: AlignItems) -> Self {
        let place = LayoutStyle::default()
            .position(Position::Absolute)
            .inset_top(0.0)
            .inset_left(0.0);
        self.layout(place)
            .map_node(move |node| float(node, placement(Some(side), align, true)))
    }

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

    /// Lets the user select and copy the node's text when `yes` is true; false leaves it unselectable.
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

    /// Sets `out` to true while the pointer is over the node and to false when it leaves.
    #[must_use]
    fn track_hovered(self, out: Signal<bool>) -> Self {
        self.map_node(move |node| on::track_hovered(node, out))
    }

    /// Sets `out` to true while focus is on the node or one of its children and to false when focus leaves.
    #[must_use]
    fn track_focused(self, out: Signal<bool>) -> Self {
        self.map_node(move |node| on::track_focused(node, out))
    }

    /// Sets `out` to true while the primary pointer button is held down on the node, without making the node clickable.
    #[must_use]
    fn track_pressed(self, out: Signal<bool>) -> Self {
        self.map_node(move |node| on::track_pressed(node, out))
    }

    /// Runs `handler` for each phase of a primary-button drag that starts on the node; the node keeps the pointer until the drag ends.
    #[must_use]
    fn pointer_drag(self, handler: impl FnMut(DragPhase, &mut PointerEvent) + 'static) -> Self {
        self.map_node(move |node| on::pointer_drag(node, handler))
    }

    /// Runs `handler` with the zoom delta of each trackpad pinch over the node; a positive delta zooms in.
    #[must_use]
    fn pinch(self, handler: impl Fn(f64, &mut GestureEvent) + 'static) -> Self {
        self.map_node(move |node| on::pinch(node, handler))
    }

    /// Runs `handler` with the angle in degrees of each trackpad rotation over the node; a positive angle turns counterclockwise.
    #[must_use]
    fn rotation(self, handler: impl Fn(f32, &mut GestureEvent) + 'static) -> Self {
        self.map_node(move |node| on::rotation(node, handler))
    }

    /// Sets the mouse cursor shown while the pointer is over the node and over children that set none.
    #[must_use]
    fn cursor(self, cursor: impl Reactive<CursorIcon>) -> Self {
        self.map_node(move |node| on::cursor(node, cursor))
    }

    /// Sets whether pointer hit tests and focus moves can land on the node; false lets pointer events pass through to what lies below.
    #[must_use]
    fn hittable(self, yes: bool) -> Self {
        self.map_node(move |node| node.hittable(yes))
    }

    /// Runs `handler` for each screen-reader edit or value request on the node; it returns true when it handled the request.
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

    /// Places the input method's candidate window at `caret`, a caret rectangle in the node's own coordinates.
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
