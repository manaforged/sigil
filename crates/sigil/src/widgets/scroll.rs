#![allow(clippy::missing_const_for_fn)]

use crate::geometry::{Point, Size};
use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::{LayoutStyle, Length, Overflow};
use crate::reactive::{Reactive, Signal, effect_named, memo, signal};
use crate::style::{Style, StyleAtom, Token, space};
use crate::widgets::Widget;

use super::layout_ext::LayoutOverrides;
use super::shared::{ViewportSizing, caller_style};

mod bar;
pub(crate) mod geometry;
mod horizontal;
mod input;

pub(crate) use bar::{THUMB_ATOMS, TRACK_ATOMS, scrollbar};
pub(crate) use horizontal::{Horizontal, probe as horizontal_probe};
pub(crate) use input::{ScrollAxis, WheelAxis, contain_scroll, pin_to_bottom, wheel};

const VIEWPORT_HEIGHT: f32 = space(60);
pub(crate) const SCROLLBAR_WIDTH: f32 = space(2);

pub(crate) const THUMB_REST_WIDTH: f32 = space(1);
pub(crate) const THUMB_MIN_HEIGHT: f32 = space(6);
const SCROLL_LINE: f32 = space(10);
const MIN_SCROLL_RANGE: f32 = 1.0;
static VIEWPORT_ATOMS: &[StyleAtom] = &[Token::Surface.background()];

/// Builder of a scroll area.
#[derive(Debug)]
#[must_use]
pub struct ScrollBuilder {
    content: NodeBuilder,
    extent: Option<Signal<f32>>,
    fit: bool,
    scroll_y: Option<Signal<f32>>,
    scroll_x: Option<Signal<f32>>,
    follow_end: bool,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A vertical scroll area around `content`, with a scrollbar; `scroll_x` adds the horizontal axis.
pub fn scroll(content: impl Into<NodeBuilder>) -> ScrollBuilder {
    ScrollBuilder {
        content: content.into(),
        extent: None,
        fit: false,
        scroll_y: None,
        scroll_x: None,
        follow_end: false,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl ScrollBuilder {
    /// Sets the content height in px; without it, the scroll area measures `content`.
    pub fn extent(mut self, extent: impl Reactive<f32>) -> Self {
        self.extent = Some(extent.into_signal());
        self
    }

    /// Makes the scroll area as high as its content, up to the height of its parent.
    pub const fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// Binds the vertical scroll offset in px to `scroll_y`.
    pub const fn scroll_y(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    /// Lets the content scroll sideways too and binds the horizontal scroll offset in px to `scroll_x`.
    pub const fn scroll_x(mut self, scroll_x: Signal<f32>) -> Self {
        self.scroll_x = Some(scroll_x);
        self
    }

    /// Keeps the view at the end as the content grows, if it is at the end, when `on` is true.
    pub const fn follow_end(mut self, on: bool) -> Self {
        self.follow_end = on;
        self
    }

    fn root(self) -> NodeBuilder {
        let scroll_y = self.scroll_y.unwrap_or_else(|| signal(0.0_f32));
        let (content, sideways) = widen(self.content, self.scroll_x);
        let (content, extent) = resolve_extent(content, self.extent);
        let set = self.layout_overrides.apply(LayoutStyle::default());
        let sizing = ViewportSizing::of(self.fit, self.layout_overrides.fills_height());
        let measured = sizing.measured(set.data());
        let width = self.layout_overrides.width_hint();
        let height = self.layout_overrides.height_hint().unwrap_or_else(|| default_height(sizing));
        let (viewport, viewport_height) = viewport_signals(width, height, measured);

        let max_scroll = memo(move || (extent.get() - viewport_height.get()).max(0.0));
        if self.follow_end {
            scroll_y.set(max_scroll.peek());
            pin_to_bottom(scroll_y, max_scroll, || true);
        }
        let fill = sizing != ViewportSizing::Fit;
        let vertical = ScrollAxis {
            offset: scroll_y,
            max: max_scroll,
            viewport: viewport_height,
        };
        let floor = if fill { viewport_height } else { extent };
        let body = wrap_content(content, layer_position(vertical, sideways), extent, floor);
        let track = scrollbar(scroll_y, max_scroll, extent, viewport_height, fill);
        let root = viewport_container(height, sizing);
        let root = match sideways {
            Some(hs) => hs.around(root.child(hs.inside(body, vertical)).child(track)),
            None => root.child(body).child(track),
        };
        let scrollable = overflows(max_scroll);
        let root = root
            .focusable(true)
            .mutate(move |node| node.a11y.tab_stop = Some(scrollable));
        let mut root = input::bind(root, scroll_y, max_scroll, viewport_height).with(
            crate::events::ScrollReveal {
                scroll_y,
                max: max_scroll,
            },
        );
        if measured {
            root = root.track_size(viewport);
        }
        caller_style(root, self.style_accumulator)
    }
}

const fn default_height(sizing: ViewportSizing) -> f32 {
    match sizing {
        ViewportSizing::Fit => 0.0,
        ViewportSizing::Fill | ViewportSizing::Fixed => VIEWPORT_HEIGHT,
    }
}

fn widen(
    content: NodeBuilder,
    scroll_x: Option<Signal<f32>>,
) -> (NodeBuilder, Option<Horizontal>) {
    let Some(scroll_x) = scroll_x else {
        return (content, None);
    };
    let size = signal(Size::ZERO);
    let hs = Horizontal::bound(memo(move || size.get().width), None, scroll_x);
    let row = container()
        .layout(LayoutStyle::flex_row().fill_width())
        .child(content.grow(1.0).shrink(0.0).track_size(size));
    (row, Some(hs))
}

fn layer_position(vertical: ScrollAxis, sideways: Option<Horizontal>) -> Signal<Point> {
    memo(move || {
        let x = sideways.map_or(0.0, |hs| -hs.scroll_x.get().clamp(0.0, hs.max.get()).round());
        let y = vertical.offset.get().clamp(0.0, vertical.max.get()).round();
        Point::new(x, -y)
    })
}

fn overflows(max_scroll: Signal<f32>) -> Signal<bool> {
    memo(move || max_scroll.get() >= MIN_SCROLL_RANGE)
}

fn resolve_extent(content: NodeBuilder, extent: Option<Signal<f32>>) -> (NodeBuilder, Signal<f32>) {
    if let Some(extent) = extent {
        (content, extent)
    } else {
        let measured = signal(Size::ZERO);
        (
            content.track_size(measured),
            memo(move || measured.get().height.max(0.0)),
        )
    }
}

pub(crate) fn reveal_row(scroll_y: Signal<f32>, row: Signal<usize>, height: f32, viewport: f32) {
    effect_named("sigil::scroll::reveal_row", move || {
        let top = f32::from(u16::try_from(row.get()).unwrap_or(u16::MAX)) * height;
        let offset = scroll_y.peek();
        let next = nearest((top, height), offset, viewport);
        if (next - offset).abs() > f32::EPSILON {
            scroll_y.set(next);
        }
    });
}

pub(crate) fn nearest((top, height): (f32, f32), current: f32, viewport: f32) -> f32 {
    if top < current || height > viewport {
        top
    } else if top + height > current + viewport {
        top + height - viewport
    } else {
        current
    }
}

fn viewport_signals(width: Option<f32>, height: f32, fill: bool) -> (Signal<Size>, Signal<f32>) {
    let viewport = signal(Size::new(width.unwrap_or(0.0), height));
    let viewport_height = memo(move || {
        let measured = viewport.get().height;
        if fill && measured > 0.0 {
            measured
        } else {
            height
        }
    });
    (viewport, viewport_height)
}

builder_impls!([] ScrollBuilder: paint(style_accumulator), widget, node(pub(crate)));

impl From<ScrollBuilder> for NodeBuilder {
    fn from(builder: ScrollBuilder) -> Self {
        builder.into_node_builder()
    }
}

fn viewport_container(height: f32, sizing: ViewportSizing) -> NodeBuilder {
    let base = match sizing {
        ViewportSizing::Fill => LayoutStyle::flex_row().fill_height(),
        ViewportSizing::Fit => LayoutStyle::flex_row().max_size(Length::Auto, Length::Percent(1.0)),
        ViewportSizing::Fixed => LayoutStyle::flex_row().height(height),
    };
    container()
        .role(Role::ScrollView)
        .layout(base.fill_width().overflow_y(Overflow::Hidden))
        .class(VIEWPORT_ATOMS)
}

fn wrap_content(
    content: NodeBuilder,
    position: Signal<Point>,
    extent: Signal<f32>,
    viewport_height: Signal<f32>,
) -> NodeBuilder {
    let height = memo(move || to_i16(extent.get().max(viewport_height.get())));
    let layer = container()
        .layout(LayoutStyle::block().width(Length::Percent(1.0)))
        .offset(position)
        .child(content);
    container()
        .layout(
            LayoutStyle::block()
                .grow(1.0)
                .width(0.0)
                .height(extent.peek().max(viewport_height.peek())),
        )
        .child(layer)
        .style_reactive(move || geometry::content_atoms(height.get()))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
pub(crate) fn to_i16(px: f32) -> i16 {
    px.clamp(f32::from(i16::MIN), f32::from(i16::MAX)).round() as i16
}
