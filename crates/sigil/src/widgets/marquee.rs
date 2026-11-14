use std::rc::Rc;

use crate::animation::{Property, clock};
use crate::geometry::Size;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container, for_each};
use crate::layout::{AlignItems, Flaw, LayoutStyle, Overflow, Space};
use crate::reactive::{Signal, effect, memo, signal};
use crate::widgets::{Framed, Widget, WidgetFrame};

const DEFAULT_SPEED: f32 = 40.0;
const GAP: Space = Space::Xl;
const MAX_COPIES: usize = 16;

/// Builder of a row that scrolls sideways without end.
#[must_use]
pub struct MarqueeBuilder {
    content: Rc<dyn Fn() -> NodeBuilder>,
    speed: f32,
    frame: WidgetFrame,
}

/// A row that repeats `content` and scrolls it left without end; it pauses under the pointer and stops under reduced motion.
pub fn marquee<N: Into<NodeBuilder>>(content: impl Fn() -> N + 'static) -> MarqueeBuilder {
    MarqueeBuilder {
        content: Rc::new(move || content().into()),
        speed: DEFAULT_SPEED,
        frame: WidgetFrame::new(),
    }
}

impl MarqueeBuilder {
    /// Sets the scroll speed in pixels per second, negative to scroll right; the default is 40.
    pub const fn speed(mut self, pixels_per_second: f32) -> Self {
        self.speed = pixels_per_second;
        self
    }

    fn root(self) -> NodeBuilder {
        let (content, speed) = (self.content, self.speed);
        let viewport = signal(Size::ZERO);
        let period = signal(Size::ZERO);
        let hovered = signal(false);
        let travelled = scroll_offset(speed, period, hovered);
        let copies = memo(move || copies_to_fill(viewport.get().width, period.get().width));
        let extra = Rc::clone(&content);
        let track = container()
            .layout(LayoutStyle::flex_row().shrink(0.0))
            .allow_flaw(Flaw::Column, "the copies repeat one strip side by side")
            .drive(Property::TranslateX, move || -travelled.get())
            .child(copy(&content).track_size(period))
            .child(for_each(
                move || (1..copies.get()).collect::<Vec<usize>>(),
                |index| *index,
                move |_| copy(&extra).accessibility_hidden(true),
            ));
        let strip = container()
            .role(Role::Marquee)
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .overflow(Overflow::Hidden),
            )
            .track_size(viewport)
            .track_hovered(hovered)
            .child(track);
        self.frame.finish(strip)
    }
}

impl Framed for MarqueeBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for MarqueeBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

impl core::fmt::Debug for MarqueeBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MarqueeBuilder")
            .field("speed", &self.speed)
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

fn copy(content: &Rc<dyn Fn() -> NodeBuilder>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .padding_right(GAP)
                .shrink(0.0),
        )
        .child(content())
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a frame step of the clock fits in f32"
)]
fn scroll_offset(speed: f32, period: Signal<Size>, hovered: Signal<bool>) -> Signal<f32> {
    let seconds = clock();
    let travelled = signal(0.0_f32);
    let mut previous = None::<f64>;
    effect(move || {
        let now = seconds.get();
        let step = previous.replace(now).map_or(0.0, |before| now - before) as f32;
        let width = period.peek().width;
        if hovered.peek() || width <= 0.0 {
            return;
        }
        travelled.update(|at| *at = speed.mul_add(step, *at).rem_euclid(width));
    });
    travelled
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the count is clamped to 1..=MAX_COPIES before the cast"
)]
fn copies_to_fill(viewport: f32, period: f32) -> usize {
    if period <= 0.0 {
        return 1;
    }
    ((viewport / period).ceil() + 1.0).clamp(1.0, MAX_COPIES as f32) as usize
}
