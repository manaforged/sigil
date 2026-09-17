use std::io::{self, Write};

use sigil::animation_runtime::tick as tick_animations;
use sigil::app::{Host, LayoutError, Painted};
use sigil::events::EventRouter;
use sigil::hal::ShellEvent;
use sigil::profile;
use sigil::reactive::Runtime;

use crate::paint::{Grid, raster_into};
use crate::run::{TICK, view};
use crate::write::Measured;

const CELL: f32 = Host::CELL;

pub struct Screen {
    nodes: Vec<Painted>,
    grid: Grid,
    prev: Grid,
    fresh: bool,
    dirty: bool,
    dt: f32,
    painter: crate::graphics::Painter,
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
            dt: TICK.as_secs_f32(),
            painter: crate::graphics::Painter::default(),
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
        effect(host, router, events, self.dt);
        let layout_phase = profile::span("layout");
        Runtime::begin_layout();
        let layout = host.layout(view(cols, rows));
        drop(layout_phase);
        if layout.is_ok() {
            let _phase = profile::span("focus");
            host.repair_focus(router);
        }
        let paint_phase = profile::span("tui.paint");
        Runtime::begin_paint();
        self.dirty = false;
        if layout.is_ok() {
            let resized = self.grid.cols() != cols as usize || self.grid.rows() != rows as usize;
            let moved = host.repaint(&mut self.nodes);
            self.dirty = moved || resized || self.fresh;
            if self.dirty {
                let _phase = profile::span("tui.raster");
                self.grid.resize(cols, rows);
                raster_into(&mut self.grid, &self.nodes, CELL);
            }
        }
        Runtime::end_frame();
        drop(paint_phase);
        profile::count_add("tui.dirty", u64::from(self.dirty));
        profile::count_add("tui.nodes", self.nodes.len() as u64);
        layout?;
        Ok(if self.dirty { &self.grid } else { &self.prev })
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

fn effect(
    host: &mut Host,
    router: &mut EventRouter,
    events: impl IntoIterator<Item = ShellEvent>,
    dt: f32,
) {
    let _phase = profile::span("effect");
    Runtime::begin_effect();
    for event in events {
        profile::interact("input");
        profile::count("tui.events");
        host.route(router, event);
    }
    tick_animations(dt);
    crate::timer::advance(dt);
    crate::work::collect_work();
}
