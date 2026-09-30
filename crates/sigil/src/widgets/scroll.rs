#![allow(clippy::missing_const_for_fn)]

use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{Dimension, LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow};
use crate::reactive::{Signal, derived, effect_named, signal};
use crate::style::{ClassSource, NodeStyleExt, Style, StyleAccess, StyleAtom, Token, space};

mod bar;
pub(crate) mod geometry;
mod input;

pub(crate) use bar::{THUMB_ATOMS, TRACK_ATOMS, scrollbar};
pub(crate) use input::{ScrollAxis, WheelAxis, contain_scroll, pin_to_bottom, wheel};

const VIEWPORT_HEIGHT: f32 = space(60);
pub(crate) const SCROLLBAR_WIDTH: f32 = space(2);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sizing {
    Fixed,
    Fill,
    Fit,
}

impl Sizing {
    fn floor(self, extent: Signal<f32>, viewport: Signal<f32>) -> Signal<f32> {
        if self == Self::Fit { extent } else { viewport }
    }
}

pub(crate) const THUMB_REST_WIDTH: f32 = space(1);
pub(crate) const THUMB_MIN_HEIGHT: f32 = space(6);
const SCROLL_LINE: f32 = space(10);
const MIN_SCROLL_RANGE: f32 = 1.0;
static VIEWPORT_ATOMS: &[StyleAtom] = &[Token::Surface.bg()];

pub struct ScrollBuilder {
    content: NodeBuilder,
    extent: Option<Signal<f32>>,
    height: f32,
    width: Option<f32>,
    sizing: Sizing,
    scroll_y: Option<Signal<f32>>,
    initial_end: bool,
    pin_bottom: bool,
    extra: Option<ClassSource>,
    style_accumulator: Style,
}

pub trait IntoScrollExtent {
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

#[must_use]
pub fn scroll(content: NodeBuilder) -> ScrollBuilder {
    ScrollBuilder {
        content,
        extent: None,
        height: VIEWPORT_HEIGHT,
        width: None,
        sizing: Sizing::Fixed,
        scroll_y: None,
        initial_end: false,
        pin_bottom: false,
        extra: None,
        style_accumulator: Style::new(),
    }
}

impl ScrollBuilder {
    #[must_use]
    pub fn extent(mut self, extent: impl IntoScrollExtent) -> Self {
        self.extent = Some(extent.into_extent());
        self
    }

    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        self.extra = Some(source.into());
        self
    }

    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.height = px;
        self
    }

    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    #[must_use]
    pub const fn fill(mut self) -> Self {
        self.sizing = Sizing::Fill;
        self
    }

    #[must_use]
    pub const fn fit(mut self) -> Self {
        self.sizing = Sizing::Fit;
        self.height = 0.0;
        self
    }

    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    #[must_use]
    pub const fn scroll_to_end(mut self) -> Self {
        self.initial_end = true;
        self
    }

    #[must_use]
    pub const fn auto_pin_to_bottom(mut self, on: bool) -> Self {
        self.pin_bottom = on;
        self
    }

    pub(crate) fn into_node_builder(self) -> NodeBuilder {
        let scroll_y = self.scroll_y.unwrap_or_else(|| signal(0.0_f32));
        let (content, extent) = resolve_extent(self.content, self.extent);
        let height = self.height;
        let width = self.width;
        let sizing = self.sizing;
        let (viewport, viewport_height) = viewport_signals(width, height, sizing != Sizing::Fixed);

        let max_scroll = derived(move || (extent.get() - viewport_height.get()).max(0.0));
        if self.initial_end {
            scroll_y.set(max_scroll.peek());
        }
        if self.pin_bottom {
            pin_to_bottom(scroll_y, max_scroll);
        }
        let floor = sizing.floor(extent, viewport_height);
        let body = wrap_content(content, scroll_y, max_scroll, extent, floor);
        let track = scrollbar(
            scroll_y,
            max_scroll,
            extent,
            viewport_height,
            sizing != Sizing::Fit,
        );

        let root = viewport_container(width, height, sizing)
            .child(body)
            .child(track);
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
        if sizing != Sizing::Fixed {
            root = root.size_reactive(viewport);
        }
        if let Some(classes) = self.extra {
            root = root.class(classes);
        }
        if !self.style_accumulator.is_empty() {
            root = root.style(self.style_accumulator);
        }
        root
    }
}

fn overflows(max_scroll: Signal<f32>) -> Signal<bool> {
    derived(move || max_scroll.get() >= MIN_SCROLL_RANGE)
}

fn resolve_extent(content: NodeBuilder, extent: Option<Signal<f32>>) -> (NodeBuilder, Signal<f32>) {
    if let Some(extent) = extent {
        (content, extent)
    } else {
        let measured = signal((0.0_f32, 0.0_f32));
        (
            content.size_reactive(measured),
            derived(move || measured.get().1.max(0.0)),
        )
    }
}

pub(crate) fn reveal_row(scroll_y: Signal<f32>, row: Signal<usize>, height: f32, viewport: f32) {
    effect_named("sigil::scroll::reveal_row", move || {
        let top = f32::from(u16::try_from(row.get()).unwrap_or(u16::MAX)) * height;
        let offset = scroll_y.peek();
        if top < offset {
            scroll_y.set(top);
        } else if top + height > offset + viewport {
            scroll_y.set(top + height - viewport);
        }
    });
}

fn viewport_signals(
    width: Option<f32>,
    height: f32,
    fill: bool,
) -> (Signal<(f32, f32)>, Signal<f32>) {
    let viewport = signal((width.unwrap_or(0.0), height));
    let viewport_height = derived(move || {
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
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn viewport_container(width: Option<f32>, height: f32, sizing: Sizing) -> NodeBuilder {
    let base = match sizing {
        Sizing::Fill => LayoutStyle::flex_row().fill_height(),
        Sizing::Fit => LayoutStyle::flex_row().max_size(Dimension::Auto, Dimension::Percent(1.0)),
        Sizing::Fixed => LayoutStyle::flex_row().height(height),
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
    let height = derived(move || to_i16(extent.get().max(viewport_height.get())));
    let position = derived(move || (0.0, -scroll_y.get().clamp(0.0, max_scroll.get()).round()));
    let layer = container()
        .layout(LayoutStyle::block().width(Dimension::Percent(1.0)))
        .position_reactive(position)
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

pub(crate) fn to_i16(px: f32) -> i16 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    {
        px.clamp(f32::from(i16::MIN), f32::from(i16::MAX)).round() as i16
    }
}
