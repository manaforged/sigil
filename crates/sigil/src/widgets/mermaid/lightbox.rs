use crate::animation::Property;
use crate::events::{ScrollDelta, ScrollEvent};
use crate::icons::Icon;
use crate::kernel::{NodeBuilder, container, text};
use crate::layout::Space;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Overflow};
use crate::reactive::{Signal, effect, memo, signal, untrack};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string_once;
use crate::style::{ControlSize, Token};
use crate::widgets::interaction::{DragEnd, pointer_drag};
use crate::widgets::{Layout, ModalBuilder, Widget, button, modal};

use super::picture;

const CHROME: (f32, f32) = (72.0, 156.0);
const MIN_AREA: f32 = 120.0;
const STEP: f32 = 1.25;
const ZOOM: (f32, f32) = (0.25, 8.0);
const PIXELS_PER_STEP: f32 = 120.0;

#[derive(Clone, Copy)]
struct Camera {
    zoom: Signal<f32>,
    x: Signal<f32>,
    y: Signal<f32>,
}

impl Camera {
    fn new() -> Self {
        Self {
            zoom: signal(1.0),
            x: signal(0.0),
            y: signal(0.0),
        }
    }

    fn scale(self, steps: f32) {
        self.zoom
            .set((self.zoom.peek() * STEP.powf(steps)).clamp(ZOOM.0, ZOOM.1));
    }

    fn fit(self) {
        self.zoom.set(1.0);
        self.x.set(0.0);
        self.y.set(0.0);
    }

    fn wheel(self, event: &mut ScrollEvent) {
        let (dx, dy) = match event.delta {
            ScrollDelta::Pixels(point) => (point.x, point.y),
            ScrollDelta::Lines(lines) => (lines.x * 40.0, lines.y * 40.0),
        };
        if event.modifiers.ctrl() || event.modifiers.meta() {
            self.scale(dy / PIXELS_PER_STEP);
        } else {
            self.x.set(self.x.peek() + dx);
            self.y.set(self.y.peek() + dy);
        }
        event.stop_propagation();
    }
}

pub(super) fn lightbox(open: Signal<bool>, source: &str, name: &str) -> ModalBuilder {
    let camera = Camera::new();
    let owned = source.to_string();
    effect(move || {
        if open.get() {
            untrack(|| camera.fit());
        }
    });
    let stage = container()
        .layout(LayoutStyle::flex_column())
        .children_reactive(move || {
            if open.get() {
                vec![self::stage(&owned, camera)]
            } else {
                Vec::new()
            }
        });
    modal(open)
        .title(name.to_string())
        .child(stage)
        .footer(controls(camera))
}

fn stage(source: &str, camera: Camera) -> NodeBuilder {
    let Some((picture, (width, height))) = picture::figure(source) else {
        return container().child(text("This diagram cannot be drawn."));
    };
    let window = crate::app::viewport_size();
    let area_width = memo(move || (window.get().width - CHROME.0).max(MIN_AREA));
    let area_height = memo(move || (window.get().height - CHROME.1).max(MIN_AREA));
    let fit = memo(move || (area_width.get() / width).min(area_height.get() / height));
    let zoom = camera.zoom;
    let drawn_width = memo(move || width * fit.get() * zoom.get());
    let drawn_height = memo(move || height * fit.get() * zoom.get());
    let picture = picture
        .drive(Property::Width, drawn_width)
        .drive(Property::Height, drawn_height)
        .drive(Property::TranslateX, camera.x)
        .drive(Property::TranslateY, camera.y);
    let area = container()
        .layout(
            LayoutStyle::flex_row()
                .overflow(Overflow::Hidden)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .drive(Property::Width, area_width)
        .drive(Property::Height, area_height)
        .hittable(true)
        .scroll(move |event| camera.wheel(event))
        .child(picture);
    pan(area, camera)
}

fn pan(area: NodeBuilder, camera: Camera) -> NodeBuilder {
    pointer_drag(
        area,
        move |event| {
            let at = event.window_position;
            Some((at.x - camera.x.peek(), at.y - camera.y.peek()))
        },
        move |&(left, top), event| {
            let at = event.window_position;
            camera.x.set(at.x - left);
            camera.y.set(at.y - top);
            event.stop_propagation();
        },
        |_: DragEnd<(f32, f32)>, _| {},
    )
}

fn controls(camera: Camera) -> NodeBuilder {
    let zoom = camera.zoom;
    let percent = memo(move || format!("{:.0}%", zoom.get() * 100.0));
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(Space::Sm)
                .align_items(AlignItems::Center),
        )
        .child(
            button(Icon::ZoomOut)
                .activate(move || camera.scale(-1.0))
                .accessible(theme_string_once(StringKey::ZoomOut))
                .size(ControlSize::Compact),
        )
        .child(
            text(percent)
                .class("text-sm")
                .class(Token::OnSurfaceMuted.text()),
        )
        .child(
            button(Icon::ZoomIn)
                .activate(move || camera.scale(1.0))
                .accessible(theme_string_once(StringKey::ZoomIn))
                .size(ControlSize::Compact),
        )
        .child(
            button("Fit")
                .activate(move || camera.fit())
                .size(ControlSize::Compact),
        )
}
