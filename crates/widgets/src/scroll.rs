//! Scroll widget — viewport clip with reactive scroll position.
//!
//! Content is offset by a negative `MarginTop` driven by a `Signal<f32>`.
//! Viewport clipping relies on layout `overflow: hidden`; visual clipping
//! is deferred to a future renderer integration pass.

#![allow(clippy::missing_const_for_fn)]

use sigil_events::prelude::NodeEventsExt;
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow, Position};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{ClassSource, NodeStyleExt, Style, StyleAccess, StyleAtom};

mod geometry;
mod input;

pub(crate) use input::{pin_to_bottom, wheel};

const VIEWPORT_HEIGHT: f32 = 240.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const THUMB_MIN_HEIGHT: f32 = 24.0;
const SCROLL_LINE: f32 = 40.0;
static VIEWPORT_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
// Thumb paints `surface-pressed-strong` — visibly darker than the
// track's `border` background. Earlier the thumb shared the track's
// `border` token, leaving the gutter readable as a single uniform
// strip with no visible thumb affordance. The strong-pressed step
// clears the same-hue JND against `border` at 8 px gutter scale.
static THUMB_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-pressed-strong")];

/// Builder returned by [`scroll`].
pub struct ScrollBuilder {
    content: NodeBuilder,
    extent: Signal<f32>,
    height: f32,
    width: Option<f32>,
    fill: bool,
    scroll_y: Option<Signal<f32>>,
    initial_end: bool,
    pin_bottom: bool,
    extra: Option<ClassSource>,
    style_accumulator: Style,
}

/// Coerce a content-extent value into a [`Signal<f32>`].
///
/// Implemented for `f32` (wraps in a fresh signal) and `Signal<f32>`
/// (passes through). Lets `scroll(content, 800.0)` and
/// `scroll(content, my_signal)` both compile.
pub trait IntoScrollExtent {
    /// Return the extent as a reactive signal in logical pixels.
    fn into_extent(self) -> Signal<f32>;
}

impl IntoScrollExtent for f32 {
    fn into_extent(self) -> Signal<f32> {
        signal(self)
    }
}

impl IntoScrollExtent for Signal<f32> {
    fn into_extent(self) -> Signal<f32> {
        self
    }
}

/// Create a scroll widget that clips `content` to a viewport.
///
/// `extent` is the total height of the content in logical pixels.
/// Pass a `Signal<f32>` to keep scroll bounds reactive (the widget
/// re-clamps as content height changes), or a bare `f32` for static
/// content.
#[must_use]
pub fn scroll(content: NodeBuilder, extent: impl IntoScrollExtent) -> ScrollBuilder {
    ScrollBuilder {
        content,
        extent: extent.into_extent(),
        height: VIEWPORT_HEIGHT,
        width: None,
        fill: false,
        scroll_y: None,
        initial_end: false,
        pin_bottom: false,
        extra: None,
        style_accumulator: Style::new(),
    }
}

impl ScrollBuilder {
    /// Apply utility classes to the viewport container root.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        self.extra = Some(source.into());
        self
    }

    /// Set the visible viewport height in pixels.
    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }

    /// Set the visible viewport width in pixels.
    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// Fill the parent's resolved height and use that measured height for
    /// scroll bounds, page steps, and scrollbar geometry.
    #[must_use]
    pub const fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Bind the viewport to a caller-owned vertical scroll offset.
    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Seed the viewport at the bottom before its first paint.
    #[must_use]
    pub const fn scroll_to_end(mut self) -> Self {
        self.initial_end = true;
        self
    }

    /// Follow content growth while the viewport is already at the bottom.
    /// Manual scrolling upward disables following until the user returns.
    #[must_use]
    pub const fn auto_pin_to_bottom(mut self, on: bool) -> Self {
        self.pin_bottom = on;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let scroll_y = self.scroll_y.unwrap_or_else(|| signal(0.0_f32));
        let extent = self.extent;
        let height = self.height;
        let width = self.width;
        let fill = self.fill;
        let content = self.content;
        let (viewport, viewport_height) = viewport_signals(width, height, fill);

        let max_scroll = derived_eq(move || (extent.get() - viewport_height.get()).max(0.0));
        if self.initial_end {
            scroll_y.set(max_scroll.peek());
        }
        if self.pin_bottom {
            pin_to_bottom(scroll_y, max_scroll);
        }
        let body = wrap_content(content, scroll_y, max_scroll, extent, viewport_height);
        let thumb = scrollbar_thumb(scroll_y, max_scroll, extent, viewport_height);
        // The thumb lives inside the track so it can use inset_right
        // against that parent, not the viewport.
        let track = scrollbar_track(height, fill).child(thumb);

        let root = viewport_container(width, height, fill)
            .child(body)
            .child(track);
        let mut root = input::bind(root, scroll_y, max_scroll, viewport_height);
        if fill {
            root = root.size_reactive(viewport);
        }
        if let Some(classes) = self.extra {
            root = root.class(classes);
        }
        // Styled accumulator installed AFTER `.class()` extras so the
        // trait surface layers ON TOP of the explicit class call
        // (additive composition per DR-049 §10). Order matches batch 1.
        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

fn viewport_signals(
    width: Option<f32>,
    height: f32,
    fill: bool,
) -> (Signal<(f32, f32)>, Signal<f32>) {
    let viewport = signal((width.unwrap_or(0.0), height));
    let viewport_height = derived_eq(move || {
        let measured = viewport.get().1;
        if fill && measured > 0.0 {
            measured
        } else {
            height
        }
    });
    (viewport, viewport_height)
}

impl StyleAccess for ScrollBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for ScrollBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn viewport_container(width: Option<f32>, height: f32, fill: bool) -> NodeBuilder {
    let base = if fill {
        LayoutStyle::flex_row().fill_height()
    } else {
        LayoutStyle::flex_row().height(height)
    };
    let base = match width {
        Some(px) => base.width(px),
        None => base.fill_width(),
    };
    container()
        .role(Role::ScrollView)
        .layout(base.overflow_y(Overflow::Hidden))
        .class(VIEWPORT_ATOMS)
}

fn wrap_content(
    content: NodeBuilder,
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    extent: Signal<f32>,
    viewport_height: Signal<f32>,
) -> NodeBuilder {
    let geometry = derived_eq(move || {
        let clamped = scroll_y.get().clamp(0.0, max_scroll.get());
        (
            to_i16(clamped),
            to_i16(extent.get().max(viewport_height.get())),
        )
    });
    // Wrap the user's content in a fresh anonymous sizing container so
    // that the user's own LayoutStyle (flex_column, padding, gap, etc.)
    // is preserved verbatim. Previously this called `.layout(…)` directly
    // on `content`, which replaced any previously-attached LayoutStyle
    // because NodeBuilder::with::<T>() routes through attach::<T>() which
    // replaces on TypeId collision. Option B: the sizing box owns the
    // fixed_size; content is added as its child unchanged.
    container()
        .layout(
            LayoutStyle::block()
                .grow(1.0)
                .width(0.0)
                .height(extent.peek().max(viewport_height.peek())),
        )
        .child(content)
        .style_reactive(move || geometry::content_atoms(geometry.get()))
}

fn scrollbar_track(height: f32, fill: bool) -> NodeBuilder {
    let layout = if fill {
        LayoutStyle::block().width(SCROLLBAR_WIDTH).fill_height()
    } else {
        LayoutStyle::block().fixed_size(SCROLLBAR_WIDTH, height)
    };
    container()
        .layout(layout)
        .class(&[StyleAtom::BackgroundThemeRef("border")] as &[_])
}

fn scrollbar_thumb(
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    extent: Signal<f32>,
    viewport_height: Signal<f32>,
) -> NodeBuilder {
    let geometry = derived_eq(move || {
        let (thumb_h, max_travel) = geometry::thumb_metrics(extent.get(), viewport_height.get());
        let max = max_scroll.get();
        let t = if max > 0.0 {
            scroll_y.get().clamp(0.0, max) / max
        } else {
            0.0
        };
        (to_i16(thumb_h), to_i16(t * max_travel))
    });

    // Track-relative drag: record the down-position and the
    // scroll value at that moment, then translate subsequent
    // pointer-move deltas via the thumb-to-content ratio. Using
    // raw event.position.y would snap the thumb to the global
    // coordinate, which is wrong whenever the scroll widget isn't
    // anchored at window y=0.
    let drag_anchor = signal::<Option<(f32, f32)>>(None);

    // Thumb is absolute-positioned against the viewport (the
    // viewport_container is the closest positioned ancestor in the
    // sigil layout model). Pin it to the right gutter at
    // `width - SCROLLBAR_WIDTH` left inset; the reactive
    // `margin_top_atoms` slides it vertically as scroll progresses.
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SCROLLBAR_WIDTH, THUMB_MIN_HEIGHT)
                .position(Position::Absolute)
                .inset_left(0.0)
                .inset_top(0.0),
        )
        .class(THUMB_ATOMS)
        .style_reactive(move || geometry::thumb_atoms(geometry.get()))
        .pointer_down(move |ev| {
            ev.capture_pointer();
            drag_anchor.set(Some((ev.position.y, scroll_y.peek())));
        })
        .pointer_up(move |_| {
            drag_anchor.set(None);
        })
        .pointer_move(move |ev| {
            if !ev.is_captured() {
                return;
            }
            let Some((anchor_y, anchor_scroll)) = drag_anchor.peek() else {
                return;
            };
            let max = max_scroll.peek();
            let (_, max_travel) = geometry::thumb_metrics(extent.peek(), viewport_height.peek());
            if max_travel <= 0.0 || max <= 0.0 {
                return;
            }
            let drag_dy = ev.position.y - anchor_y;
            let offset_dy = drag_dy * (max / max_travel);
            let next = (anchor_scroll + offset_dy).clamp(0.0, max);
            scroll_y.set(next);
            ev.stop_propagation();
        })
}

fn to_i16(px: f32) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        px.clamp(f32::from(i16::MIN), f32::from(i16::MAX)).round() as i16
    }
}
