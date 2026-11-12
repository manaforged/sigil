use std::hint::black_box;

use ratatui::backend::{Backend, CrosstermBackend, TestBackend};
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

use crate::measure::{Change, Rig};
use crate::scene::{BODY, COLS, LINES, WHEEL_ROWS, extend, titles};

const LINES_PER_MESSAGE: usize = 4;

/// How much of the transcript one ratatui frame lays out.
#[derive(Clone, Copy)]
pub enum Extent {
    Visible,
    Whole,
}

/// Where a ratatui frame goes, without a real terminal.
pub trait Output: Backend + Sized {
    fn open() -> Terminal<Self>;

    fn fit(terminal: &mut Terminal<Self>, area: Rect);

    fn drain(&mut self) -> usize;

    fn shown(&mut self) -> String;
}

impl Output for TestBackend {
    fn open() -> Terminal<Self> {
        Terminal::new(Self::new(COLS, LINES)).expect("terminal")
    }

    fn fit(terminal: &mut Terminal<Self>, area: Rect) {
        terminal.backend_mut().resize(area.width, area.height);
    }

    fn drain(&mut self) -> usize {
        0
    }

    fn shown(&mut self) -> String {
        self.to_string()
    }
}

fn written(out: Vec<u8>, area: Rect) -> Terminal<CrosstermBackend<Vec<u8>>> {
    let viewport = Viewport::Fixed(area);
    Terminal::with_options(CrosstermBackend::new(out), TerminalOptions { viewport })
        .expect("terminal")
}

impl Output for CrosstermBackend<Vec<u8>> {
    fn open() -> Terminal<Self> {
        written(Vec::new(), Rect::new(0, 0, COLS, LINES))
    }

    fn fit(terminal: &mut Terminal<Self>, area: Rect) {
        let out = std::mem::take(terminal.backend_mut().writer_mut());
        *terminal = written(out, area);
    }

    fn drain(&mut self) -> usize {
        let bytes = self.writer().len();
        self.writer_mut().clear();
        bytes
    }

    fn shown(&mut self) -> String {
        format!("written bytes {}", self.drain())
    }
}

pub struct RatatuiRig<B: Output> {
    terminal: Terminal<B>,
    titles: Vec<String>,
    stream: String,
    offset: u16,
    extent: Extent,
}

fn view(frame: &mut Frame<'_>, titles: &[String], stream: &str, offset: u16, extent: Extent) {
    let area = frame.area();
    let mut lines = Vec::with_capacity(titles.len() * LINES_PER_MESSAGE + 1);
    for title in titles {
        lines.push(Line::raw(title.as_str()));
        lines.push(Line::default());
        lines.push(Line::raw(BODY));
        lines.push(Line::default());
    }
    lines.push(Line::raw(stream));
    let transcript = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((offset, 0));
    match extent {
        Extent::Visible => frame.render_widget(transcript, area),
        Extent::Whole => {
            let total = transcript.line_count(area.width);
            let mut bar = ScrollbarState::new(total)
                .position(usize::from(offset))
                .viewport_content_length(usize::from(area.height));
            frame.render_widget(transcript, area);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut bar,
            );
        }
    }
}

impl<B: Output> RatatuiRig<B> {
    pub fn new(extent: Extent) -> Self {
        Self {
            terminal: B::open(),
            titles: titles(),
            stream: String::new(),
            offset: 0,
            extent,
        }
    }

    fn draw(&mut self) {
        let Self {
            terminal,
            titles,
            stream,
            offset,
            extent,
        } = self;
        let done = terminal
            .draw(|frame| view(frame, titles, stream, *offset, *extent))
            .expect("draw");
        black_box(done.count);
    }
}

impl<B: Output> Rig for RatatuiRig<B> {
    fn frame(&mut self, change: Change) {
        match change {
            Change::Grow => extend(&mut self.stream),
            Change::Wheel(lines) => {
                let rows = (-lines * WHEEL_ROWS) as i16;
                self.offset = self.offset.saturating_add_signed(rows);
            }
            _ => {}
        }
        if let Some(narrow) = change.narrow() {
            let area = Rect::new(0, 0, COLS - u16::from(narrow), LINES);
            B::fit(&mut self.terminal, area);
        }
        self.draw();
        black_box(self.terminal.backend_mut().drain());
    }

    fn show(&mut self) -> String {
        self.draw();
        self.terminal.backend_mut().shown()
    }
}
