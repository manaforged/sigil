use crate::animation::{Property, Transition};
use crate::layout::{LayoutStyle, Orientation, cols, rows};
use crate::node::{NodeBuilder, container, text};
use crate::reactive::{Signal, memo};
use crate::style::{
    BORDER_WIDTH, Background, Color, Elevation, GradientAxis, Metrics, Paint, Radius, Token, Tone,
};
use crate::widgets::Widget;

use super::builder::SliderRuntime;
use super::handlers::ThumbSignals;

const THUMB_SIZE: f32 = 16.0;
const TRACK_THICKNESS: f32 = 6.0;
const CELL_THUMB: &str = "●";

pub(super) fn track(
    runtime: SliderRuntime,
    extent: Signal<crate::geometry::Size>,
    filled: bool,
) -> NodeBuilder {
    if filled {
        segment(
            runtime,
            extent,
            true,
            Some(Box::new(move || runtime.ratio())),
        )
    } else {
        segment(runtime, extent, false, None)
    }
}

pub(super) fn segment(
    runtime: SliderRuntime,
    extent: Signal<crate::geometry::Size>,
    filled: bool,
    ratio: Option<Box<dyn Fn() -> f32>>,
) -> NodeBuilder {
    let thickness = Metrics::current().track_size(TRACK_THICKNESS);
    ratio.map_or_else(
        || {
            let layout = match runtime.orientation {
                Orientation::Horizontal => LayoutStyle::block().grow(1.0).height(thickness),
                Orientation::Vertical => LayoutStyle::block().grow(1.0).width(thickness),
            };
            styled_segment(runtime.glass, filled).layout(layout)
        },
        |ratio| match runtime.orientation {
            Orientation::Horizontal => {
                let px = memo(move || ratio().clamp(0.0, 1.0) * extent.get().width);
                styled_segment(runtime.glass, filled)
                    .layout(LayoutStyle::block().height(thickness))
                    .drive(Property::Width, px)
            }
            Orientation::Vertical => {
                let px = memo(move || ratio().clamp(0.0, 1.0) * extent.get().height);
                styled_segment(runtime.glass, filled)
                    .layout(LayoutStyle::block().width(thickness))
                    .drive(Property::Height, px)
            }
        },
    )
}

fn styled_segment(glass: bool, filled: bool) -> NodeBuilder {
    let base = container().hittable(false).radius(Radius::Full);
    match (filled, glass) {
        (true, true) => base.background(
            Background::linear(GradientAxis::TopToBottom)
                .start(Token::AccentHover)
                .end(Token::Accent),
        ),
        (true, false) => base.background(Token::Accent),
        (false, true) => base
            .background(
                Background::linear(GradientAxis::TopToBottom)
                    .start(Color::rgba(148, 163, 184, 66))
                    .middle(Color::rgba(148, 163, 184, 30))
                    .end(Color::rgba(148, 163, 184, 46)),
            )
            .border_width(BORDER_WIDTH)
            .border_color(Color::rgba(148, 163, 184, 90)),
        (false, false) => base.background(Tone::Neutral.step(4)),
    }
}

pub(super) fn slider_thumb(
    signals: ThumbSignals,
    disabled: Option<Signal<bool>>,
    glass: bool,
) -> NodeBuilder {
    if Metrics::current().is_grid() {
        cell_thumb()
    } else {
        window_thumb(signals, disabled, glass)
    }
}

fn cell_thumb() -> NodeBuilder {
    container()
        .hittable(false)
        .layout(
            LayoutStyle::block()
                .width(cols(1.0))
                .height(rows(1.0))
                .shrink(0.0),
        )
        .text_color(Token::Accent)
        .child(text(CELL_THUMB).hittable(false))
}

fn window_thumb(signals: ThumbSignals, disabled: Option<Signal<bool>>, glass: bool) -> NodeBuilder {
    let active = memo(move || {
        (signals.hovered.get()
            || signals.focused.get()
            || signals.pressed.get()
            || signals.dragging.get())
            && !disabled.is_some_and(Signal::get)
    });
    let node = container()
        .hittable(false)
        .layout(LayoutStyle::block().fixed_size(THUMB_SIZE, THUMB_SIZE))
        .radius(Radius::Lg)
        .transition(Transition::smooth())
        .border_width(BORDER_WIDTH);
    if glass {
        let node = node
            .background(
                Background::linear(GradientAxis::TopToBottom)
                    .start(Color::rgba(255, 255, 255, 250))
                    .middle(Color::rgba(240, 243, 248, 235))
                    .end(Color::rgba(214, 219, 230, 240)),
            )
            .border_color(Color::rgba(255, 255, 255, 140))
            .paint_when(active, |s| s.border_color(Color::rgba(255, 255, 255, 190)));
        match disabled {
            Some(flag) => node
                .paint_when(memo(move || !flag.get()), |s| s.shadow(Elevation::Md))
                .paint_when(flag, |s| s.border_color(Color::rgba(255, 255, 255, 60))),
            None => node.shadow(Elevation::Md),
        }
    } else {
        let node = node
            .background(Token::OnAccent)
            .border_color(Token::Accent)
            .paint_when(active, |s| s.border_color(Token::AccentHover));
        match disabled {
            Some(flag) => node
                .paint_when(memo(move || !flag.get()), |s| s.shadow(Elevation::Sm))
                .paint_when(flag, |s| s.border_color(Tone::Neutral.step(8))),
            None => node.shadow(Elevation::Sm),
        }
    }
}
