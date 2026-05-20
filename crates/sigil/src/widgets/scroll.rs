//! Scroll widget — viewport clip with reactive scroll position.
//!
//! Content is offset by a negative `MarginTop` driven by a `Signal<f32>`.
//! Viewport clipping relies on layout `overflow: hidden`; visual clipping
//! is deferred to a future renderer integration pass.

#![allow(clippy::missing_const_for_fn)]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt, ScrollDelta, ScrollEvent};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Overflow, Position};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{ClassSource, NodeStyleExt, Px, StateHost, Style, StyleAccess, StyleAtom};

const VIEWPORT_WIDTH: f32 = 320.0;
const VIEWPORT_HEIGHT: f32 = 240.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const THUMB_MIN_HEIGHT: f32 = 24.0;
const SCROLL_LINE: f32 = 40.0;
const ATOM_CACHE_CAP: usize = 256;

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
    width: f32,
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
        width: VIEWPORT_WIDTH,
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
        self.width = px;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let scroll_y = signal(0.0_f32);
        let extent = self.extent;
        let height = self.height;
        let width = self.width;
        let content = self.content;

        let max_scroll = derived_eq(move || (extent.get() - height).max(0.0));
        let initial = extent.peek().max(height);
        let body = wrap_content(content, scroll_y, max_scroll, initial, width);
        let track = scrollbar_track(height);
        let thumb = scrollbar_thumb(scroll_y, max_scroll, initial, height, width);

        let mut root = viewport_container(width, height)
            .child(body)
            .child(track)
            .child(thumb)
            .key(Key::ArrowDown, move |ev| {
                step(ev, scroll_y, SCROLL_LINE, max_scroll.peek());
            })
            .key(Key::ArrowUp, move |ev| {
                step(ev, scroll_y, -SCROLL_LINE, max_scroll.peek());
            })
            .key(Key::PageDown, move |ev| {
                step(ev, scroll_y, height, max_scroll.peek());
            })
            .key(Key::PageUp, move |ev| {
                step(ev, scroll_y, -height, max_scroll.peek());
            })
            .key(Key::Home, move |ev| {
                jump(ev, scroll_y, 0.0);
            })
            .key(Key::End, move |ev| {
                jump(ev, scroll_y, max_scroll.peek());
            })
            .on_scroll(move |ev| {
                wheel(ev, scroll_y, height, max_scroll.peek());
            });
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

impl StyleAccess for ScrollBuilder {
    fn style(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for ScrollBuilder {}

impl IntoNode for ScrollBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn viewport_container(width: f32, height: f32) -> NodeBuilder {
    container()
        .role(Role::ScrollView)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(width, height)
                .overflow_y(Overflow::Hidden),
        )
        .class(VIEWPORT_ATOMS)
}

fn wrap_content(
    content: NodeBuilder,
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    initial: f32,
    width: f32,
) -> NodeBuilder {
    let offset_px = derived_eq(move || {
        let clamped = scroll_y.get().clamp(0.0, max_scroll.get());
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = clamped.round() as i16;
        px
    });
    // Layout box snaps to initial extent; if content grows past it,
    // the overflow:hidden viewport still clips correctly — only the
    // body's own inner layout reflows less ideally. Tracked: make
    // body fixed_size reactive once the layout system grows a
    // Height style atom.
    content
        .layout(LayoutStyle::block().fixed_size(width - SCROLLBAR_WIDTH, initial))
        .style_reactive(move || margin_top_atoms(offset_px.get()))
}

fn scrollbar_track(height: f32) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(SCROLLBAR_WIDTH, height))
        .class(&[StyleAtom::BackgroundThemeRef("border")] as &[_])
}

fn scrollbar_thumb(
    scroll_y: Signal<f32>,
    max_scroll: Signal<f32>,
    initial: f32,
    height: f32,
    width: f32,
) -> NodeBuilder {
    let ratio = (height / initial).clamp(0.0, 1.0);
    let thumb_h = (ratio * height).max(THUMB_MIN_HEIGHT);
    let max_travel = (height - thumb_h).max(0.0);

    let thumb_top = derived_eq(move || {
        let max = max_scroll.get();
        let t = if max > 0.0 {
            scroll_y.get().clamp(0.0, max) / max
        } else {
            0.0
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = (t * max_travel).round() as i16;
        px
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
    let thumb_left = width - SCROLLBAR_WIDTH;
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SCROLLBAR_WIDTH, thumb_h)
                .position(Position::Absolute)
                .inset_left(thumb_left)
                .inset_top(0.0),
        )
        .class(THUMB_ATOMS)
        .style_reactive(move || margin_top_atoms(thumb_top.get()))
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

/// Apply a fixed pixel step (arrow / page keys). Only stops
/// propagation when scroll actually moved, so the parent can take
/// over at the bounds.
fn step(ev: &mut KeyEvent, scroll_y: Signal<f32>, delta: f32, max: f32) {
    let current = scroll_y.peek();
    let next = (current + delta).clamp(0.0, max);
    if (next - current).abs() > f32::EPSILON {
        scroll_y.set(next);
        ev.stop_propagation();
    }
}

/// Jump to an absolute position (Home / End).
fn jump(ev: &mut KeyEvent, scroll_y: Signal<f32>, target: f32) {
    ev.stop_propagation();
    scroll_y.set(target);
}

/// Translate a wheel / trackpad delta into a scroll-offset update.
///
/// Pixel deltas apply directly (they already reflect user intent at
/// device granularity). Line deltas multiply by `SCROLL_LINE` and
/// clamp to the viewport so a runaway delta cannot skip the whole
/// document in one event. Propagation only stops when the scroll
/// position changes; at top/bottom the event bubbles so an outer
/// scroll container can take over.
fn wheel(ev: &mut ScrollEvent, scroll_y: Signal<f32>, height: f32, max: f32) {
    let dy = match ev.delta {
        ScrollDelta::Pixels(p) => p.y,
        ScrollDelta::Lines(l) => (l.y * SCROLL_LINE).clamp(-height, height),
    };
    // Wheel-down (positive y on most platforms) means the user
    // wants to see content lower in the document, i.e. the scroll
    // offset should increase. Sign-invert the platform delta.
    let current = scroll_y.peek();
    let next = (current - dy).clamp(0.0, max);
    if (next - current).abs() > f32::EPSILON {
        scroll_y.set(next);
        ev.stop_propagation();
    }
}

fn margin_top_atoms(px: i16) -> &'static [StyleAtom] {
    cached_atoms(px, || vec![StyleAtom::MarginTop(Px(px.wrapping_neg()))])
}

fn cached_atoms(key: i16, build: impl FnOnce() -> Vec<StyleAtom>) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = cache.lock() else {
        return Box::leak(build().into_boxed_slice());
    };
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let leaked: &'static [StyleAtom] = Box::leak(build().into_boxed_slice());
    if map.len() < ATOM_CACHE_CAP {
        map.insert(key, leaked);
    }
    leaked
}
