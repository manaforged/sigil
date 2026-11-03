use std::cell::RefCell;
use std::io::{self, Write as _};
use std::rc::Rc;

use rayon::prelude::*;
use tiny_skia::Pixmap;

use crate::app::{FrameHooks, FrameInfo};
use crate::events::{Event, WindowEvent};
use crate::geometry::{LogicalSize, PhysicalSize, Point, Rect, Scale};
use crate::kernel::{HostMode, IntoNode};
use crate::profile::{self, Phase};
use crate::renderer::damage::{GPU_BUDGET, culled};
use crate::renderer::{CpuRenderer, SURFACE_CLEAR};
use crate::scene::CommandScene;
use crate::scene::damage::{Damage, scene_damage};
use crate::style::srgb::decode;
use crate::style::{Color, Rgba8};
use crate::terminal::graphics::Mux;
use crate::terminal::pixels::tiles::{Grid, Tiles};
use crate::terminal::run::CELL;
use crate::terminal::term::Term;
use crate::{AppResult, Error, Host};

const ROW: f32 = 18.0;

thread_local! {
    static RENDERER: RefCell<CpuRenderer> = RefCell::new(CpuRenderer::new());
}

/// The pixel host and the renderer that turns its frames into Kitty images.
pub(super) struct Draw {
    host: Host,
    fresh: bool,
    last: Option<Frame>,
    canvas: Option<Pixmap>,
    scene: Option<CommandScene>,
    tiles: Tiles,
}

impl Draw {
    pub(super) fn mount<N: IntoNode>(tree: &Rc<impl Fn() -> N + 'static>) -> Self {
        let mut host = Host::new(HostMode::Pixel);
        let shared = tree.clone();
        host.mount_app(move || shared());
        Self {
            host,
            fresh: true,
            last: None,
            canvas: None,
            scene: None,
            tiles: Tiles::default(),
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
        stdout.write_all(b"\x1b[?25l\x1b[2J")?;
        stdout.flush()
    }

    /// Deletes the images so the cell host can draw again.
    pub(super) fn leave(&mut self, mux: Option<Mux>) -> io::Result<()> {
        self.tiles.clear(mux)
    }

    /// Runs one frame and draws it when it changed; returns whether the app asked to exit.
    pub(super) fn turn(
        &mut self,
        input: Vec<Event>,
        (cols, rows): (u16, u16),
        cell: [u16; 2],
        term: &mut Term,
        dt: f32,
    ) -> AppResult<bool> {
        let caps = term.caps();
        let frame = Frame::new(cols, rows, cell);
        let events: Vec<Event> = input.into_iter().map(|event| frame.remap(event)).collect();
        let force = core::mem::take(&mut self.fresh) || self.last != Some(frame);
        self.last = Some(frame);
        let mut scene = None;
        profile::begin_frame(profile::BackendStatus::TERMINAL);
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
                        let _emit = profile::span(Phase::Emit);
                        scene = Some(host.emit(frame.viewport));
                    }
                })),
                ..FrameHooks::default()
            },
        );
        out.layout?;
        if let Some(scene) = scene {
            let clear = caps.background.map_or(SURFACE_CLEAR, opaque);
            self.paint(frame, scene, force, (caps.mux, clear))?;
        }
        profile::tick_frame();
        Ok(out.exit)
    }

    /// Repaints the damaged part of the canvas and sends the tiles it touched.
    fn paint(
        &mut self,
        frame: Frame,
        scene: CommandScene,
        force: bool,
        (mux, clear): (Option<Mux>, Color),
    ) -> AppResult {
        let render = profile::span(Phase::RenderCpu);
        let scale = frame.scale();
        let area = Rect::new(Point::new(0.0, 0.0), frame.viewport.0, frame.viewport.1);
        let damage = match (&self.scene, &self.canvas) {
            (Some(prev), Some(_)) if !force => {
                scene_damage::<Vec<Rect>>((prev, &scene), (area, scale), GPU_BUDGET)
            }
            _ => Damage::Full(crate::scene::damage::FullReason::ViewportResized),
        };
        let size = frame.size;
        let fits = self
            .canvas
            .as_ref()
            .is_some_and(|c| c.width() == size.width && c.height() == size.height);
        if !fits {
            self.canvas = Pixmap::new(size.width, size.height);
        }
        let Some(canvas) = self.canvas.as_mut() else {
            return Ok(());
        };
        let rects = match damage {
            Damage::Unchanged if fits => {
                self.scene = Some(scene);
                return Ok(());
            }
            Damage::Regions(rects) if fits => Some(rects),
            _ => None,
        };
        let regions: Vec<[usize; 4]> = match &rects {
            Some(rects) => rects.iter().map(|rect| frame.pixels(*rect)).collect(),
            None => vec![[0, 0, size.width as usize, size.height as usize]],
        };
        let jobs: Vec<[usize; 4]> = regions
            .iter()
            .flat_map(|region| frame.strips(*region))
            .collect();
        let painted: Vec<([usize; 4], Option<Pixmap>)> = jobs
            .par_iter()
            .map(|&job| {
                let culled = culled(&scene, frame.logical(job));
                let rect = job.map(|n| u32::try_from(n).unwrap_or(u32::MAX));
                let pixmap = RENDERER.with_borrow_mut(|renderer| {
                    renderer.render_region(&culled, (frame.viewport, clear), size, rect)
                });
                (job, pixmap)
            })
            .collect();
        for (job, pixmap) in &painted {
            if let Some(pixmap) = pixmap {
                place(canvas, pixmap, *job);
            }
        }
        let dirty = rects.is_some().then_some(regions);
        drop(render);
        self.scene = Some(scene);
        let Some(canvas) = &self.canvas else {
            return Ok(());
        };
        self.tiles
            .send(frame.grid, canvas, dirty.as_deref(), mux)
            .map_err(Error::io)
    }
}

/// The opaque linear color of an sRGB byte color.
fn opaque(color: Rgba8) -> Color {
    Color::rgba(decode(color.r), decode(color.g), decode(color.b), 1.0)
}

#[derive(Clone, Copy, PartialEq)]
struct Frame {
    grid: Grid,
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
            grid: Grid {
                cols,
                rows,
                cell: [width, height],
            },
            cell,
            viewport: (f32::from(cols) * cell[0], f32::from(rows) * cell[1]),
            size,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "a frame width in pixels fits the f32 mantissa"
    )]
    fn scale(&self) -> f32 {
        self.size.width as f32 / self.viewport.0.max(1.0)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "damage rects are clamped to the non-negative frame"
    )]
    fn pixels(&self, rect: Rect) -> [usize; 4] {
        let scale = self.scale();
        let x = (rect.origin.x * scale).floor().max(0.0) as usize;
        let y = (rect.origin.y * scale).floor().max(0.0) as usize;
        let right = ((rect.origin.x + rect.width) * scale).ceil().max(0.0) as usize;
        let bottom = ((rect.origin.y + rect.height) * scale).ceil().max(0.0) as usize;
        let right = right.min(self.size.width as usize);
        let bottom = bottom.min(self.size.height as usize);
        let (x, y) = (x.min(right), y.min(bottom));
        [x, y, right - x, bottom - y]
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel coordinates of a frame fit the f32 mantissa"
    )]
    fn logical(&self, [x, y, w, h]: [usize; 4]) -> Rect {
        let scale = self.scale();
        Rect::new(
            Point::new(x as f32 / scale, y as f32 / scale),
            w as f32 / scale,
            h as f32 / scale,
        )
    }

    /// Cuts a pixel rect at tile-row boundaries so its parts paint in parallel.
    fn strips(&self, [x, y, w, h]: [usize; 4]) -> impl Iterator<Item = [usize; 4]> {
        let band = (usize::from(self.grid.cell[1]) * usize::from(self.grid.tile()[1])).max(1);
        let bottom = y + h;
        let mut top = y;
        std::iter::from_fn(move || {
            (top < bottom).then(|| {
                let end = ((top / band + 1) * band).min(bottom);
                let strip = [x, top, w, end - top];
                top = end;
                strip
            })
        })
        .filter(|[_, _, w, h]| *w > 0 && *h > 0)
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
}

fn place(canvas: &mut Pixmap, region: &Pixmap, [x, y, w, _]: [usize; 4]) {
    let stride = canvas.width() as usize * 4;
    let row = w * 4;
    for (line, source) in region.data().chunks_exact(row).enumerate() {
        let at = (y + line) * stride + x * 4;
        if let Some(dest) = canvas.data_mut().get_mut(at..at + row) {
            dest.copy_from_slice(source);
        }
    }
}
