use std::io::{self, Write};

use crate::app::{FrameHooks, Host, LayoutError};
use crate::events::EventRouter;
use crate::hal::ShellEvent;
use crate::profile;
use crate::scene::CommandScene;

use crate::terminal::access::{Access, Announcer, Place, Speech};
use crate::terminal::caps::Caps;
use crate::terminal::paint::{Grid, Marks, Paint, raster_with};
use crate::terminal::run::view;
use crate::terminal::transcript::Transcript;
use crate::terminal::write::Measured;

const CELL: f32 = Host::CELL;

pub struct Screen {
    scene: CommandScene,
    grid: Grid,
    prev: Grid,
    fresh: bool,
    dirty: bool,
    exited: bool,
    dt: f32,
    painter: crate::terminal::graphics::Painter,
    transcript: Transcript,
    caps: Caps,
    live_rows: usize,
    announcer: Announcer,
}

impl Screen {
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            scene: CommandScene::default(),
            grid: Grid::new(cols, rows),
            prev: Grid::new(cols, rows),
            fresh: true,
            dirty: true,
            exited: false,
            dt: crate::app::FRAME_INTERVAL.as_secs_f32(),
            painter: crate::terminal::graphics::Painter::default(),
            transcript: Transcript::from_env(),
            caps: Caps::default(),
            live_rows: 0,
            announcer: Announcer::default(),
        }
    }

    #[must_use]
    pub fn with_caps(cols: u16, rows: u16, caps: Caps) -> Self {
        let mut screen = Self::new(cols, rows);
        screen.set_caps(caps);
        screen
    }

    pub const fn set_caps(&mut self, caps: Caps) {
        self.caps = caps;
        self.grid.set_graphics(caps.graphics.is_some());
        self.prev.set_graphics(caps.graphics.is_some());
        self.fresh = true;
    }

    pub const fn set_access(&mut self, access: Access) {
        self.announcer.access = access;
    }

    pub(crate) const fn speak_above(&mut self, yes: bool) {
        self.announcer.speech = if yes {
            Speech::Above
        } else {
            Speech::StatusRow
        };
    }

    pub(crate) fn take_above(&mut self) -> Vec<String> {
        self.announcer.take_above()
    }

    pub(crate) fn deadline(&self) -> Option<std::time::Instant> {
        self.announcer.deadline()
    }

    #[must_use]
    pub const fn caps(&self) -> &Caps {
        &self.caps
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
            profile::interact(crate::profile::Cause::Resize);
        }
        profile::count(profile::Counter::TuiFrames);
        profile::count_add(profile::Counter::WorkloadCols, u64::from(cols));
        profile::count_add(profile::Counter::WorkloadRows, u64::from(rows));
        if crate::hud::take_state_change() {
            self.fresh = true;
        }
        let dt = self.dt;
        let caps = self.caps;
        let Self {
            scene,
            grid,
            fresh,
            dirty,
            ..
        } = self;
        let cells = Cells {
            scene,
            grid,
            fresh,
            dirty,
            caps,
        };
        let hooks = frame_hooks(cells, events, (cols, rows));
        let out = host.frame(router, view(cols, rows), dt, hooks);
        self.exited |= out.exit;
        if crate::hud::enabled() {
            crate::terminal::hud::draw(&mut self.grid);
            if !self.dirty && !self.fresh && !self.unchanged() {
                self.dirty = true;
            }
        }
        let update = host.accessibility_tree(router);
        self.listen(&update);
        if self.dirty && !self.fresh && self.unchanged() {
            self.dirty = false;
            core::mem::swap(&mut self.prev, &mut self.grid);
        }
        if self.dirty {
            self.transcript.emit(&update, &self.grid);
        }
        profile::count_add(profile::Counter::TuiDirty, u64::from(self.dirty));
        profile::count_add(profile::Counter::TuiNodes, self.scene.commands.len() as u64);
        out.layout?;
        Ok(if self.dirty { &self.grid } else { &self.prev })
    }

    fn listen(&mut self, update: &accesskit::TreeUpdate) {
        let reading = self.transcript.read(update, CELL);
        let changed = self.announcer.hear(&reading);
        let raw = self.dirty;
        if raw {
            self.announcer.caret = self.grid.cursor().is_some();
        }
        let focus = self.announcer.focus_cell(&self.grid, &reading);
        if !raw && (changed || matches!(focus, Place::Move(at) if at != self.prev.cursor())) {
            self.grid.clone_from(&self.prev);
            self.dirty = true;
        }
        if self.dirty {
            self.announcer.apply(&mut self.grid, raw, focus);
        }
    }

    fn unchanged(&self) -> bool {
        self.prev.cols() == self.grid.cols()
            && self.prev.rows() == self.grid.rows()
            && self.prev.cursor() == self.grid.cursor()
            && self.prev.images() == self.grid.images()
            && (0..self.grid.rows()).all(|row| {
                crate::terminal::write::same_row(self.prev.row(row), self.grid.row(row), &self.caps)
            })
    }

    #[must_use]
    pub const fn exited(&self) -> bool {
        self.exited
    }

    #[must_use]
    pub fn text(&self) -> String {
        if self.dirty { &self.grid } else { &self.prev }.text()
    }

    pub fn flush(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !profile::enabled() {
            return self.write(out);
        }
        let _phase = profile::span(profile::Phase::TuiWrite);
        let mut measured = Measured::new(out);
        let result = self.write(&mut measured);
        profile::count_add(profile::Counter::TuiBytes, measured.bytes);
        profile::count_add(profile::Counter::TuiWrites, measured.writes);
        result
    }

    pub fn flush_inline(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !profile::enabled() {
            return self.write_inline(out);
        }
        let _phase = profile::span(profile::Phase::TuiWrite);
        let mut measured = Measured::new(out);
        let result = self.write_inline(&mut measured);
        profile::count_add(profile::Counter::TuiBytes, measured.bytes);
        profile::count_add(profile::Counter::TuiWrites, measured.writes);
        result
    }

    fn write_inline(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        let prev = (!self.fresh).then_some(&self.prev);
        crate::terminal::write::inline(
            out,
            prev,
            &self.grid,
            (self.live_rows, self.announcer.caret),
            &self.caps,
        )?;
        self.live_rows = self.grid.rows();
        self.fresh = false;
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    fn write(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        let full = self.fresh
            || self.prev.cols() != self.grid.cols()
            || self.prev.rows() != self.grid.rows();
        let tracked = if self.caps.graphics.is_some() || profile::enabled() {
            self.grid.rows()
        } else {
            0
        };
        let rewritten: Vec<bool> = (0..tracked)
            .map(|row| {
                full || !crate::terminal::write::same_row(
                    self.prev.row(row),
                    self.grid.row(row),
                    &self.caps,
                )
            })
            .collect();
        profile::count_add(
            profile::Counter::TuiRowsWritten,
            rewritten.iter().filter(|row| **row).count() as u64,
        );
        if full {
            self.fresh = false;
            self.prev.mirror(&self.grid);
            crate::terminal::write::all(out, &self.grid, &self.caps)?;
        } else {
            crate::terminal::write::dirty(out, &self.prev, &self.grid, &self.caps)?;
        }
        self.painter.emit(
            out,
            self.caps,
            crate::terminal::graphics::Frame {
                next: self.grid.images(),
                rewritten: &rewritten,
                redraw: full,
            },
        )?;
        crate::terminal::write::cursor(out, self.grid.cursor(), self.announcer.caret, &self.caps)?;
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    #[must_use]
    pub const fn rows(&self) -> usize {
        self.grid.rows()
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
    scene: &'s mut CommandScene,
    grid: &'s mut Grid,
    fresh: &'s mut bool,
    dirty: &'s mut bool,
    caps: Caps,
}

fn frame_hooks<'s>(
    cells: Cells<'s>,
    events: impl IntoIterator<Item = ShellEvent> + 's,
    size: (u16, u16),
) -> FrameHooks<'s> {
    let Cells {
        scene,
        grid,
        fresh,
        dirty,
        caps,
    } = cells;
    let (cols, rows) = size;
    FrameHooks {
        input: Some(Box::new(move |host, router| {
            let _phase = profile::span(profile::Phase::Effect);
            for event in events {
                profile::interact(profile::Cause::Input);
                profile::count(profile::Counter::TuiEvents);
                host.route(router, event);
            }
        })),
        paint: Some(Box::new(move |host, router, info| {
            let _phase = profile::span(profile::Phase::TuiPaint);
            if !info.layout_ok {
                return;
            }
            let resized = grid.cols() != usize::from(cols) || grid.rows() != usize::from(rows);
            let next = host.emit(view(cols, rows));
            *dirty = next != *scene || resized || *fresh;
            if *dirty {
                let _raster = profile::span(profile::Phase::TuiRaster);
                grid.resize(cols, rows);
                let marks = Marks::collect(host.cx(), router.focus(), &next, CELL);
                let paint = Paint {
                    theme: host.theme(),
                    cell: CELL,
                    caps,
                    marks: &marks,
                };
                raster_with(grid, &next, &paint);
                *scene = next;
            }
        })),
        ..FrameHooks::default()
    }
}
