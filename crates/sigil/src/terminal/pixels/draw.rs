use std::io::{self, Write as _};
use std::rc::Rc;

use image::ImageEncoder as _;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use crate::app::{FrameHooks, FrameInfo};
use crate::events::{Event, WindowEvent};
use crate::geometry::{LogicalSize, PhysicalSize, Point, Scale};
use crate::kernel::{HostMode, IntoNode};
use crate::renderer::CpuRenderer;
use crate::scene::CommandScene;
use crate::style::Color;
use crate::terminal::graphics::{CHUNK, Mux, base64, write_mux};
use crate::terminal::run::CELL;
use crate::terminal::term::Term;
use crate::{AppResult, Error, ErrorKind, Host};

const IMAGE: u32 = 0x5167_0001;
const ROW: f32 = 18.0;
const CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

/// The pixel host and the renderer that turns its frames into Kitty images.
pub(super) struct Draw {
    host: Host,
    renderer: CpuRenderer,
    fresh: bool,
    last: Option<Frame>,
}

impl Draw {
    pub(super) fn mount<N: IntoNode>(tree: &Rc<impl Fn() -> N + 'static>) -> Self {
        let mut host = Host::new(HostMode::Pixel);
        let shared = tree.clone();
        host.mount_app(move || shared());
        Self {
            host,
            renderer: CpuRenderer::new(),
            fresh: true,
            last: None,
        }
    }

    pub(super) const fn host(&self) -> &Host {
        &self.host
    }

    pub(super) const fn host_mut(&mut self) -> &mut Host {
        &mut self.host
    }

    /// Clears the cell text that the image would leave visible through its transparent parts.
    pub(super) fn enter(&mut self) -> io::Result<()> {
        self.fresh = true;
        let mut stdout = io::stdout().lock();
        stdout.write_all(b"\x1b[2J")?;
        stdout.flush()
    }

    /// Deletes the image so the cell host can draw again.
    pub(super) fn leave(&mut self, mux: Option<Mux>) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        write_mux(
            &mut stdout,
            &format!("\x1b_Ga=d,d=I,i={IMAGE},q=2\x1b\\"),
            mux,
        )?;
        stdout.flush()
    }

    /// Runs one frame and draws it when it changed; returns whether the app asked to exit.
    pub(super) fn turn(
        &mut self,
        input: Vec<Event>,
        (cols, rows): (u16, u16),
        term: &mut Term,
        dt: f32,
    ) -> AppResult<bool> {
        let caps = term.caps();
        let Some(cell) = caps.cell else {
            return Err(Error::fixed(
                ErrorKind::Software,
                "terminal stopped reporting its cell size",
            ));
        };
        let frame = Frame::new(cols, rows, cell);
        let events: Vec<Event> = input.into_iter().map(|event| frame.remap(event)).collect();
        let force = core::mem::take(&mut self.fresh) || self.last != Some(frame);
        self.last = Some(frame);
        let mut scene = None;
        let out = self.host.frame(
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
        out.layout?;
        if let Some(scene) = scene {
            frame.draw(&mut self.renderer, &scene, caps.mux)?;
        }
        Ok(out.exit)
    }
}

#[derive(Clone, Copy, PartialEq)]
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
        let mut stdout = io::stdout().lock();
        stdout.write_all(&out).map_err(Error::io)?;
        stdout.flush().map_err(Error::io)
    }
}
