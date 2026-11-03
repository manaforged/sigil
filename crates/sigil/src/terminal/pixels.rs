use std::io::Write as _;
use std::time::Instant;

use image::ImageEncoder as _;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use crate::app::{FrameHooks, FrameInfo};
use crate::events::{Event, WindowEvent};
use crate::geometry::{LogicalSize, PhysicalSize, Point, Scale};
use crate::kernel::{HostMode, IntoNode};
use crate::renderer::CpuRenderer;
use crate::scene::CommandScene;
use crate::style::Color;
use crate::tasks::AsyncRuntime;
use crate::terminal::caps::Caps;
use crate::terminal::graphics::{CHUNK, Mux, Protocol, base64, write_mux};
use crate::terminal::input::{Incoming, InputWake, gather_input};
use crate::terminal::run::{CELL, Hooks, frame_due, pump, turn_wait};
use crate::terminal::term::Term;
use crate::{AppConfig, AppResult, Error, ErrorKind, Host};

const ENV: &str = "SIGIL_TERMINAL";
const IMAGE: u32 = 0x5167_0001;
const ROW: f32 = 18.0;
const CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

/// Returns whether `SIGIL_TERMINAL=pixels` asks for pixel mode and the terminal can show it.
pub(super) fn wanted(config: &AppConfig, caps: Caps) -> bool {
    if std::env::var(ENV).as_deref() != Ok("pixels") {
        return false;
    }
    let fits = !config.inline
        && caps.graphics == Some(Protocol::Kitty)
        && caps.cell.is_some_and(|[w, h]| w > 0 && h > 0);
    if !fits {
        tracing::warn!(
            target: "sigil::terminal",
            "{ENV}=pixels needs the kitty graphics protocol and a reported cell size; drawing cells"
        );
    }
    fits
}

/// Runs the app with the window layout, drawn by the CPU renderer into one kitty image.
pub(super) fn live<N: IntoNode>(
    config: &AppConfig,
    tree: impl Fn() -> N + 'static,
    (async_rt, waker): (&mut AsyncRuntime, &InputWake),
    term: &mut Term,
    (mut cols, mut rows): (u16, u16),
    hooks: &mut Hooks<'_>,
) -> AppResult {
    let mut host = Host::new(HostMode::Pixel);
    host.mount_app(tree);
    let mut renderer = CpuRenderer::new();
    let mut pointer = Point::new(0.0, 0.0);
    let (mut first, mut fresh, mut last) = (true, true, Instant::now());
    let result = loop {
        if crate::terminal::signals::stopped() {
            break Ok(());
        }
        let woke = pump(&host, async_rt);
        let timing = turn_wait(config.frame_interval, woke, last, None);
        let input = match gather_input(&mut first, config, &mut pointer, hooks, (waker, timing))? {
            Incoming::Quit => break Ok(()),
            Incoming::Idle if !fresh && woke == 0 && !frame_due() => continue,
            Incoming::Idle => Vec::new(),
            Incoming::Events(events) => events,
            Incoming::Resized(next_cols, next_rows, events) => {
                (cols, rows) = (next_cols, next_rows);
                term.mark_cell_stale();
                fresh = true;
                events
            }
        };
        fresh |= term.remeasure().is_some();
        let caps = term.caps();
        let Some(cell) = caps.cell else {
            break Err(Error::fixed(
                ErrorKind::Software,
                "terminal stopped reporting its cell size",
            ));
        };
        let frame = Frame::new(cols, rows, cell);
        let events: Vec<Event> = input.into_iter().map(|event| frame.remap(event)).collect();
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let force = core::mem::take(&mut fresh);
        let mut scene = None;
        let out = host.frame(
            frame.viewport,
            dt,
            FrameHooks {
                input: Some(Box::new(move |host: &mut Host| {
                    for event in events {
                        host.route(event);
                    }
                    None
                })),
                paint: Some(Box::new(|host: &mut Host, info: &FrameInfo| {
                    if force || info.dirty || info.animating {
                        scene = Some(host.emit(frame.viewport));
                    }
                })),
                ..FrameHooks::default()
            },
        );
        if let Err(err) = out.layout {
            break Err(err);
        }
        if let Some(scene) = scene {
            frame.draw(&mut renderer, &scene, caps.mux)?;
        }
        if out.exit {
            break Ok(());
        }
    };
    let erase = format!("\x1b_Ga=d,d=I,i={IMAGE},q=2\x1b\\");
    let mut stdout = std::io::stdout().lock();
    write_mux(&mut stdout, &erase, term.caps().mux).map_err(Error::io)?;
    stdout.flush().map_err(Error::io)?;
    result
}

#[derive(Clone, Copy)]
struct Frame {
    cols: u16,
    rows: u16,
    cell: [f32; 2],
    viewport: (f32, f32),
    size: PhysicalSize,
}

impl Frame {
    fn new(cols: u16, rows: u16, [width, height]: [u16; 2]) -> Self {
        let scale = f32::from(height) / ROW;
        let size = PhysicalSize::new(
            u32::from(cols) * u32::from(width),
            u32::from(rows) * u32::from(height),
        );
        let cell = [f32::from(width) / scale, f32::from(height) / scale];
        Self {
            cols,
            rows,
            cell,
            viewport: (f32::from(cols) * cell[0], f32::from(rows) * cell[1]),
            size,
        }
    }

    fn point(&self, at: Point) -> Point {
        Point::new(at.x / CELL * self.cell[0], at.y / CELL * self.cell[1])
    }

    fn remap(&self, event: Event) -> Event {
        match event {
            Event::Pointer(mut pointer) => {
                pointer.position = self.point(pointer.position);
                pointer.window_position = self.point(pointer.window_position);
                pointer.delta = self.point(pointer.delta);
                Event::Pointer(pointer)
            }
            Event::Scroll(mut scroll) => {
                scroll.position = self.point(scroll.position);
                Event::Scroll(scroll)
            }
            Event::Window(WindowEvent::Resized(_)) => Event::Window(WindowEvent::Resized(
                Scale::ONE.to_physical(LogicalSize::new(self.viewport.0, self.viewport.1)),
            )),
            other => other,
        }
    }

    fn draw(
        &self,
        renderer: &mut CpuRenderer,
        scene: &CommandScene,
        mux: Option<Mux>,
    ) -> AppResult {
        let pixmap = renderer.render(scene, CLEAR, self.viewport, self.size)?;
        let mut rgba = Vec::with_capacity(pixmap.pixels().len() * 4);
        for px in pixmap.pixels() {
            let c = px.demultiply();
            rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
            .write_image(
                &rgba,
                self.size.width,
                self.size.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|err| {
                Error::message(
                    ErrorKind::Software,
                    format_args!("pixel frame encode failed: {err}"),
                )
            })?;
        let data = base64(&png);
        let chunks: Vec<&[u8]> = data.as_bytes().chunks(CHUNK).collect();
        let mut out = Vec::with_capacity(data.len() + chunks.len() * 16 + 64);
        out.extend_from_slice(b"\x1b[?2026h\x1b[H");
        for (index, chunk) in chunks.iter().enumerate() {
            let more = u8::from(index + 1 < chunks.len());
            let head = if index == 0 {
                format!(
                    "a=T,f=100,i={IMAGE},p=1,q=2,C=1,z=1,c={},r={},m={more}",
                    self.cols, self.rows
                )
            } else {
                format!("m={more}")
            };
            let body = std::str::from_utf8(chunk).unwrap_or_default();
            write_mux(&mut out, &format!("\x1b_G{head};{body}\x1b\\"), mux).map_err(Error::io)?;
        }
        out.extend_from_slice(b"\x1b[?2026l");
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&out).map_err(Error::io)?;
        stdout.flush().map_err(Error::io)
    }
}
