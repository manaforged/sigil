use std::hint::black_box;
use std::sync::LazyLock;

use iced_core::mouse::{self, Cursor, ScrollDelta};
use iced_core::text::Wrapping;
use iced_core::time::Instant;
use iced_core::widget::Id;
use iced_core::widget::operation::{Operation, Scrollable};
use iced_core::{
    Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Theme, Vector, clipboard,
    renderer, window,
};
use iced_runtime::user_interface::{Cache, State, UserInterface};
use iced_tiny_skia::Renderer;
use iced_widget::{Column, scrollable, text};

use crate::measure::{Change, Rig};
use crate::scene::{BODY, MESSAGES, WHEEL_LINE, extend, titles, view};

const TEXT_SIZE: f32 = 16.0;
const ICED_WHEEL_LINE: f32 = 60.0;

static TITLES: LazyLock<Vec<String>> = LazyLock::new(titles);

type Page = Element<'static, (), Theme, Renderer>;
type Interface = UserInterface<'static, (), Theme, Renderer>;

pub struct IcedRig {
    renderer: Renderer,
    interface: Option<Interface>,
    theme: Theme,
    stream: String,
    cursor: Cursor,
    narrow: bool,
}

#[derive(Default)]
struct Offset(f32);

impl Operation for Offset {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0 = translation.y;
    }
}

fn transcript(stream: &str) -> Page {
    let mut page = Column::with_capacity(MESSAGES + 1).width(Length::Fill);
    for title in TITLES.iter() {
        let message = Column::with_capacity(2)
            .push(text(title.as_str()).wrapping(Wrapping::None))
            .push(text(BODY));
        page = page.push(message);
    }
    page = page.push(text(stream.to_owned()));
    scrollable(page)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

impl IcedRig {
    pub fn new() -> Self {
        let mut rig = Self {
            renderer: Renderer::new(Font::default(), Pixels(TEXT_SIZE)),
            interface: None,
            theme: Theme::Light,
            stream: String::new(),
            cursor: Cursor::Unavailable,
            narrow: false,
        };
        rig.rebuild();
        rig
    }

    fn size(&self) -> Size {
        let (width, height) = view(self.narrow);
        Size::new(width, height)
    }

    fn rebuild(&mut self) {
        let cache = self
            .interface
            .take()
            .map_or_else(Cache::new, UserInterface::into_cache);
        let page = transcript(&self.stream);
        self.interface = Some(UserInterface::build(
            page,
            self.size(),
            cache,
            &mut self.renderer,
        ));
    }

    fn relayout(&mut self) {
        let size = self.size();
        if let Some(interface) = self.interface.take() {
            self.interface = Some(interface.relayout(size, &mut self.renderer));
        }
    }

    fn events(&mut self, change: Change) -> Vec<Event> {
        let size = self.size();
        let at = Point::new(size.width / 2.0, size.height / 2.0);
        let mut events = Vec::new();
        match change {
            Change::Point => {
                self.cursor = Cursor::Available(at);
                events.push(Event::Mouse(mouse::Event::CursorMoved { position: at }));
            }
            Change::Wheel(lines) => {
                let delta = ScrollDelta::Lines {
                    x: 0.0,
                    y: lines * WHEEL_LINE / ICED_WHEEL_LINE,
                };
                events.push(Event::Mouse(mouse::Event::WheelScrolled { delta }));
            }
            _ => {}
        }
        events.push(Event::Window(
            window::Event::RedrawRequested(Instant::now()),
        ));
        events
    }

    fn update(&mut self, events: &[Event]) -> bool {
        let Some(interface) = self.interface.as_mut() else {
            return false;
        };
        let mut messages = Vec::new();
        let (state, _) = interface.update(
            events,
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        matches!(state, State::Outdated)
    }

    fn draw(&mut self) -> usize {
        let Some(interface) = self.interface.as_mut() else {
            return 0;
        };
        interface.draw(
            &mut self.renderer,
            &self.theme,
            &renderer::Style::default(),
            self.cursor,
        );
        self.renderer.layers().len()
    }
}

impl Rig for IcedRig {
    fn frame(&mut self, change: Change) {
        if let Some(narrow) = change.narrow() {
            self.narrow = narrow;
            self.relayout();
        }
        if matches!(change, Change::Grow) {
            extend(&mut self.stream);
            self.rebuild();
        }
        let events = self.events(change);
        if self.update(&events) {
            self.rebuild();
        }
        black_box(self.draw());
    }

    fn show(&mut self) -> String {
        let layers = self.draw();
        let mut offset = Offset::default();
        if let Some(interface) = self.interface.as_mut() {
            interface.operate(&self.renderer, &mut offset);
        }
        format!("layers {layers} offset {}", offset.0)
    }
}
