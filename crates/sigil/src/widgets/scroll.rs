#![allow(clippy::missing_const_for_fn)]

use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::{Dimension, LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, Overflow};
use crate::reactive::{Signal, derived, effect_named, signal};
use crate::style::{PaintTarget, Style, StyleAtom, StyleSlot, Token, space};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

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
    sizing: Sizing,
    scroll_y: Option<Signal<f32>>,
    follow_end: bool,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
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
        sizing: Sizing::Fixed,
        scroll_y: None,
        follow_end: false,
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl ScrollBuilder {
    #[must_use]
    pub fn extent(mut self, extent: impl IntoScrollExtent) -> Self {
        self.extent = Some(extent.into_extent());
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
        self
    }

    #[must_use]
    pub const fn scroll_bind(mut self, scroll_y: Signal<f32>) -> Self {
        self.scroll_y = Some(scroll_y);
        self
    }

    #[must_use]
    pub const fn follow_end(mut self, on: bool) -> Self {
        self.follow_end = on;
        self
    }

    pub(crate) fn into_node_builder(self) -> NodeBuilder {
        let scroll_y = self.scroll_y.unwrap_or_else(|| signal(0.0_f32));
        let (content, extent) = resolve_extent(self.content, self.extent);
        let sizing = self.sizing;
        let width = self.layout_overrides.width_hint();
        let height = self.layout_overrides.height_hint().unwrap_or(match sizing {
            Sizing::Fit => 0.0,
            Sizing::Fill | Sizing::Fixed => VIEWPORT_HEIGHT,
        });
        let (viewport, viewport_height) = viewport_signals(width, height, sizing != Sizing::Fixed);

        let max_scroll = derived(move || (extent.get() - viewport_height.get()).max(0.0));
        if self.follow_end {
            scroll_y.set(max_scroll.peek());
            pin_to_bottom(scroll_y, max_scroll, || true);
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

        let root = viewport_container(self.layout_overrides, height, sizing)
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

impl PaintTarget for ScrollBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for ScrollBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for ScrollBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

fn viewport_container(overrides: LayoutOverrides, height: f32, sizing: Sizing) -> NodeBuilder {
    let base = match sizing {
        Sizing::Fill => LayoutStyle::flex_row().fill_height(),
        Sizing::Fit => LayoutStyle::flex_row().max_size(Dimension::Auto, Dimension::Percent(1.0)),
        Sizing::Fixed => LayoutStyle::flex_row().height(height),
    };
    container()
        .role(Role::ScrollView)
        .layout(overrides.apply(base.fill_width().overflow_y(Overflow::Hidden)))
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
