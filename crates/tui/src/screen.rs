use std::io::{self, Write};

use sigil::app::{FrameHooks, Host, LayoutError, Painted};
use sigil::events::EventRouter;
use sigil::hal::ShellEvent;
use sigil::profile;

use crate::paint::{Grid, raster_into};
use crate::run::{TICK, view};
use crate::transcript::Transcript;
use crate::write::Measured;

const CELL: f32 = Host::CELL;

pub struct Screen {
    nodes: Vec<Painted>,
    grid: Grid,
    prev: Grid,
    fresh: bool,
    dirty: bool,
    exited: bool,
    dt: f32,
    painter: crate::graphics::Painter,
    transcript: Transcript,
}

impl Screen {
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            nodes: Vec::new(),
            grid: Grid::new(cols, rows),
            prev: Grid::new(cols, rows),
            fresh: true,
            dirty: true,
            exited: false,
            dt: TICK.as_secs_f32(),
            painter: crate::graphics::Painter::default(),
            transcript: Transcript::from_env(),
        }
    }

    pub fn frame(
        &mut self,
        host: &mut Host,
        router: &mut EventRouter,
        events: impl IntoIterator<Item = ShellEvent>,
        size: (u16, u16),
    ) -> Result<&Grid, LayoutError> {
        let (cols, rows) = size;
        if self.grid.cols() != usize::from(cols) || self.grid.rows() != usize::from(rows) {
            profile::interact("resize");
        }
        profile::count("tui.frames");
        profile::count_add("workload.cols", u64::from(cols));
        profile::count_add("workload.rows", u64::from(rows));
        let dt = self.dt;
        let Self {
            nodes,
            grid,
            fresh,
            dirty,
            ..
        } = self;
        let cells = Cells {
            nodes,
            grid,
            fresh,
            dirty,
        };
        let hooks = frame_hooks(cells, events, (cols, rows));
        let out = host.frame(router, view(cols, rows), dt, hooks);
        self.exited |= out.exit;
        if self.dirty {
            self.transcript.emit(host, router, &self.grid);
        }
        profile::count_add("tui.dirty", u64::from(self.dirty));
        profile::count_add("tui.nodes", self.nodes.len() as u64);
        out.layout?;
        Ok(if self.dirty { &self.grid } else { &self.prev })
    }

    #[must_use]
    pub const fn exited(&self) -> bool {
        self.exited
    }

    pub fn flush(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !profile::enabled() {
            return self.write(out);
        }
        let _phase = profile::span("tui.write");
        let mut measured = Measured::new(out);
        let result = self.write(&mut measured);
        profile::count_add("tui.bytes", measured.bytes);
        profile::count_add("tui.writes", measured.writes);
        result
    }

    fn write(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        let full = self.fresh
            || self.prev.cols() != self.grid.cols()
            || self.prev.rows() != self.grid.rows();
        let rewritten: Vec<bool> = (0..self.grid.rows())
            .map(|row| full || self.prev.row(row) != self.grid.row(row))
            .collect();
        profile::count_add(
            "tui.rows_written",
            rewritten.iter().filter(|row| **row).count() as u64,
        );
        if full {
            self.fresh = false;
            self.prev.mirror(&self.grid);
            crate::write::all(out, &self.grid)?;
        } else {
            crate::write::dirty(out, &self.prev, &self.grid)?;
        }
        self.painter.emit(
            out,
            crate::graphics::Frame {
                next: self.grid.images(),
                rewritten: &rewritten,
                redraw: full,
            },
        )?;
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    pub const fn set_dt(&mut self, dt: f32) {
        self.dt = dt;
    }

    pub const fn invalidate(&mut self) {
        self.fresh = true;
    }

    #[must_use]
    pub const fn needs_flush(&self) -> bool {
        self.dirty
    }
}

struct Cells<'s> {
    nodes: &'s mut Vec<Painted>,
    grid: &'s mut Grid,
    fresh: &'s mut bool,
    dirty: &'s mut bool,
}

fn frame_hooks<'s>(
    cells: Cells<'s>,
    events: impl IntoIterator<Item = ShellEvent> + 's,
    size: (u16, u16),
) -> FrameHooks<'s> {
    let Cells {
        nodes,
        grid,
        fresh,
        dirty,
    } = cells;
    let (cols, rows) = size;
    FrameHooks {
        input: Some(Box::new(move |host, router| {
            let _phase = profile::span("effect");
            for event in events {
                profile::interact("input");
                profile::count("tui.events");
                host.route(router, event);
            }
        })),
        paint: Some(Box::new(move |host, _router, info| {
            let _phase = profile::span("tui.paint");
            if !info.layout_ok {
                return;
            }
            let resized = grid.cols() != usize::from(cols) || grid.rows() != usize::from(rows);
            let moved = host.repaint(nodes);
            *dirty = moved || resized || *fresh;
            if *dirty {
                let _raster = profile::span("tui.raster");
                grid.resize(cols, rows);
                raster_into(grid, nodes, CELL);
            }
        })),
        ..FrameHooks::default()
    }
}
