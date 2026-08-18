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
use sigil_layout::prelude::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow, Position};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{
    ClassSource, Dimension as StyleDimension, NodeStyleExt, Px, Style, StyleAccess, StyleAtom,
};

const VIEWPORT_HEIGHT: f32 = 240.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const THUMB_MIN_HEIGHT: f32 = 24.0;
const SCROLL_LINE: f32 = 40.0;
const ATOM_CACHE_CAP: usize = 256;

type AtomCache = OnceLock<Mutex<HashMap<(i16, i16), &'static [StyleAtom]>>>;

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

    fn into_node_builder(self) -> NodeBuilder {
        let scroll_y = signal(0.0_f32);
        let extent = self.extent;
        let height = self.height;
        let width = self.width;
        let fill = self.fill;
        let content = self.content;
        let (viewport, viewport_height) = viewport_signals(width, height, fill);

        let max_scroll = derived_eq(move || (extent.get() - viewport_height.get()).max(0.0));
        let body = wrap_content(content, scroll_y, max_scroll, extent, viewport_height);
        let thumb = scrollbar_thumb(scroll_y, max_scroll, extent, viewport_height);
        // The thumb lives inside the track so it can use inset_right
        // against that parent, not the viewport.
        let track = scrollbar_track(height, fill).child(thumb);

        let mut root = viewport_container(width, height, fill)
            .child(body)
            .child(track)
            .key(Key::ArrowDown, move |ev| {
                step(ev, scroll_y, SCROLL_LINE, max_scroll.peek());
            })
            .key(Key::ArrowUp, move |ev| {
                step(ev, scroll_y, -SCROLL_LINE, max_scroll.peek());
            })
            .key(Key::PageDown, move |ev| {
                step(ev, scroll_y, viewport_height.peek(), max_scroll.peek());
            })
            .key(Key::PageUp, move |ev| {
                step(ev, scroll_y, -viewport_height.peek(), max_scroll.peek());
            })
            .key(Key::Home, move |ev| {
                jump(ev, scroll_y, 0.0);
            })
            .key(Key::End, move |ev| {
                jump(ev, scroll_y, max_scroll.peek());
            })
            .on_scroll(move |ev| {
                wheel(ev, scroll_y, viewport_height.peek(), max_scroll.peek());
            });
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
        .style_reactive(move || content_geometry_atoms(geometry.get()))
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
        let (thumb_h, max_travel) = thumb_metrics(extent.get(), viewport_height.get());
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
    let thumb_left = 0.0_f32;
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SCROLLBAR_WIDTH, THUMB_MIN_HEIGHT)
                .position(Position::Absolute)
                .inset_left(thumb_left)
                .inset_top(0.0),
        )
        .class(THUMB_ATOMS)
        .style_reactive(move || thumb_geometry_atoms(geometry.get()))
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
            let (_, max_travel) = thumb_metrics(extent.peek(), viewport_height.peek());
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

fn thumb_metrics(extent: f32, viewport_height: f32) -> (f32, f32) {
    let height = viewport_height.max(0.0);
    let content = extent.max(height);
    let ratio = if content > 0.0 {
        (height / content).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let thumb_h = (ratio * height)
        .max(THUMB_MIN_HEIGHT.min(height))
        .min(height);
    (thumb_h, (height - thumb_h).max(0.0))
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
///
/// Shared with `virtual_list`, whose viewport scrolls the same way.
pub(crate) fn wheel(ev: &mut ScrollEvent, scroll_y: Signal<f32>, height: f32, max: f32) {
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

fn content_geometry_atoms((offset, height): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: AtomCache = OnceLock::new();
    cached_atoms(&CACHE, (offset, height), || {
        vec![
            StyleAtom::MarginTop(Px(offset.wrapping_neg())),
            StyleAtom::Height(StyleDimension::Px(Px(height))),
        ]
    })
}

fn thumb_geometry_atoms((height, offset): (i16, i16)) -> &'static [StyleAtom] {
    static CACHE: AtomCache = OnceLock::new();
    cached_atoms(&CACHE, (height, offset), || {
        vec![
            StyleAtom::Height(StyleDimension::Px(Px(height))),
            StyleAtom::MarginTop(Px(offset)),
        ]
    })
}

fn cached_atoms(
    cache: &'static AtomCache,
    key: (i16, i16),
    build: impl FnOnce() -> Vec<StyleAtom>,
) -> &'static [StyleAtom] {
    let cache = cache.get_or_init(|| Mutex::new(HashMap::new()));
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

fn to_i16(px: f32) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        px.clamp(f32::from(i16::MIN), f32::from(i16::MAX)).round() as i16
    }
}
