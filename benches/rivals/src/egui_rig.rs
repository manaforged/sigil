use std::hint::black_box;

use egui::{
    CentralPanel, Context, Event, FullOutput, Label, Modifiers, MouseWheelSource, MouseWheelUnit,
    Pos2, RawInput, Rect, ScrollArea, TextStyle, TextWrapMode, TouchPhase, Ui, Vec2,
};

use crate::measure::{Change, Rig};
use crate::scene::{BODY, FRAME_SECONDS, MESSAGES, extend, titles, view};

const ROWS_PER_MESSAGE: usize = 2;

/// How the transcript is built and what one frame produces.
#[derive(Clone, Copy)]
pub enum Mode {
    All,
    Shapes,
    Rows,
}

pub struct EguiRig {
    ctx: Context,
    titles: Vec<String>,
    stream: String,
    time: f64,
    narrow: bool,
    mode: Mode,
    offset: f32,
}

fn heading(ui: &mut Ui, title: &str) {
    ui.add(Label::new(title).wrap_mode(TextWrapMode::Extend));
}

fn every(ui: &mut Ui, titles: &[String], stream: &str) -> f32 {
    let area = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for title in titles {
            heading(ui, title);
            ui.label(BODY);
        }
        ui.label(stream);
    });
    area.state.clamped_offset().y
}

fn windowed(ui: &mut Ui, titles: &[String], stream: &str) -> f32 {
    let height = ui.text_style_height(&TextStyle::Body);
    let total = MESSAGES * ROWS_PER_MESSAGE + 1;
    let area = ScrollArea::vertical()
        .auto_shrink(false)
        .show_rows(ui, height, total, |ui, range| {
            for row in range {
                match titles.get(row / ROWS_PER_MESSAGE) {
                    Some(title) if row % ROWS_PER_MESSAGE == 0 => heading(ui, title),
                    Some(_) => {
                        ui.label(BODY);
                    }
                    None => {
                        ui.label(stream);
                    }
                }
            }
        });
    area.state.clamped_offset().y
}

impl EguiRig {
    pub fn new(mode: Mode) -> Self {
        Self {
            ctx: Context::default(),
            titles: titles(),
            stream: String::new(),
            time: 0.0,
            narrow: false,
            mode,
            offset: 0.0,
        }
    }

    fn size(&self) -> Vec2 {
        let (width, height) = view(self.narrow);
        Vec2::new(width, height)
    }

    fn events(&self, change: Change) -> Vec<Event> {
        let at = (self.size() / 2.0).to_pos2();
        match change {
            Change::Point => vec![Event::PointerMoved(at)],
            Change::Wheel(lines) => vec![Event::MouseWheel {
                unit: MouseWheelUnit::Line,
                delta: Vec2::new(0.0, lines),
                phase: TouchPhase::Move,
                source: MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            }],
            _ => Vec::new(),
        }
    }

    fn pass(&mut self, events: Vec<Event>) -> FullOutput {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size())),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        self.time += f64::from(FRAME_SECONDS);
        let Self {
            ctx,
            titles,
            stream,
            mode,
            offset,
            ..
        } = self;
        ctx.run_ui(input, |ui| {
            CentralPanel::default().show(ui, |ui| {
                *offset = match mode {
                    Mode::All | Mode::Shapes => every(ui, titles, stream),
                    Mode::Rows => windowed(ui, titles, stream),
                };
            });
        })
    }

    fn draw(&mut self, events: Vec<Event>) -> (usize, usize) {
        let output = self.pass(events);
        let shapes = output.shapes.len();
        let primitives = match self.mode {
            Mode::Shapes => 0,
            Mode::All | Mode::Rows => self
                .ctx
                .tessellate(output.shapes, output.pixels_per_point)
                .len(),
        };
        (shapes, primitives)
    }
}

impl Rig for EguiRig {
    fn frame(&mut self, change: Change) {
        if let Some(narrow) = change.narrow() {
            self.narrow = narrow;
        }
        if matches!(change, Change::Grow) {
            extend(&mut self.stream);
        }
        let events = self.events(change);
        black_box(self.draw(events));
    }

    fn show(&mut self) -> String {
        let (shapes, primitives) = self.draw(Vec::new());
        format!(
            "shapes {shapes} primitives {primitives} offset {}",
            self.offset
        )
    }
}
