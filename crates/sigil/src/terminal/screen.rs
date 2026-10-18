use std::io::{self, Write};

use crate::Host;
use crate::events::EventRouter;
use crate::events::ShellEvent;
use crate::layout::LayoutError;
use crate::profile;
use crate::scene::CommandScene;

use crate::terminal::access::{Access, Announcer, Place, Speech};
use crate::terminal::caps::Caps;
use crate::terminal::paint::{Grid, Marks};
use crate::terminal::run::view;
use crate::terminal::transcript::Transcript;
use crate::terminal::write::{Damage, Measured};

mod raster;

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
    damage: Option<Damage>,
    marks: Marks,
    stale: Option<Vec<usize>>,
    projection: Projection,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Projected {
    #[default]
    Never,
    Focus(Option<crate::kernel::NodeId>),
}

#[derive(Default)]
pub(super) struct Projection {
    pub(super) quiet: bool,
    last: Projected,
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
            damage: None,
            marks: Marks::default(),
            stale: None,
            projection: Projection::default(),
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
        if crate::hud::take_state_change() || self.dirty {
            self.fresh = true;
        }
        let dt = self.dt;
        self.damage = None;
        self.projection.quiet = !crate::reactive::Runtime::frame_dirty();
        let hooks = raster::frame_hooks(self, events, (cols, rows));
        let out = host.frame(router, view(cols, rows), dt, hooks);
        self.exited |= out.exit;
        if crate::hud::enabled() {
            crate::terminal::hud::draw(&mut self.grid);
            self.damage = None;
            self.stale = None;
            if !self.dirty && !self.fresh && !self.unchanged() {
                self.dirty = true;
            }
        }
        let update = self.read_tree(host, router);
        if self.dirty {
            self.stale = self
                .damage
                .as_ref()
                .map(|damage| damage.rows(usize::MAX).collect());
        }
        if self.dirty && !self.fresh && self.unchanged() {
            self.dirty = false;
            core::mem::swap(&mut self.prev, &mut self.grid);
        }
        if self.dirty
            && let Some(update) = &update
        {
            self.transcript.emit(update, &self.grid);
        }
        profile::count_add(profile::Counter::TuiDirty, u64::from(self.dirty));
        profile::count_add(profile::Counter::TuiNodes, self.scene.commands.len() as u64);
        out.layout?;
        Ok(if self.dirty { &self.grid } else { &self.prev })
    }

    fn read_tree(&mut self, host: &Host, router: &EventRouter) -> Option<accesskit::TreeUpdate> {
        let records = self.transcript.records();
        let skip = self.projection.quiet
            && self.projection.last == Projected::Focus(router.focus())
            && !records
            && self
                .announcer
                .deadline()
                .is_none_or(|due| due > std::time::Instant::now());
        if skip {
            return None;
        }
        let scope = if records {
            crate::a11y::mapping::TreeScope::Full
        } else {
            crate::a11y::mapping::TreeScope::Reading
        };
        let update = host.accessibility_tree_scoped(router, scope);
        self.projection.last = Projected::Focus(router.focus());
        self.listen(&update);
        Some(update)
    }

    fn listen(&mut self, update: &accesskit::TreeUpdate) {
        let reading = self.transcript.read(update, CELL);
        let changed = self.announcer.hear(&reading);
        let raw = self.dirty;
        if raw {
            self.announcer.caret = self.grid.cursor().is_some();
        }
        let shown = if raw { &self.grid } else { &self.prev };
        let focus = self.announcer.focus_cell(shown, &reading);
        if !raw && (changed || matches!(focus, Place::Move(at) if at != self.prev.cursor())) {
            self.grid.sync_rows(&self.prev, self.stale.as_deref());
            self.dirty = true;
            self.damage = Some(Damage::new(Vec::new(), false));
        }
        if self.dirty {
            let rows = self.announcer.apply(&mut self.grid, raw, focus);
            if let Some(damage) = &mut self.damage {
                rows.into_iter().flatten().for_each(|row| damage.touch(row));
            }
        }
    }

    fn unchanged(&self) -> bool {
        self.prev.cursor() == self.grid.cursor()
            && self.prev.images() == self.grid.images()
            && crate::terminal::write::unchanged(
                &self.prev,
                &self.grid,
                self.damage.as_ref(),
                &self.caps,
            )
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
        self.measure(out, false)
    }

    pub fn flush_inline(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.measure(out, true)
    }

    fn measure(&mut self, out: &mut impl Write, inline: bool) -> io::Result<()> {
        if !profile::enabled() {
            return self.emit(out, inline);
        }
        let _phase = profile::span(profile::Phase::TuiWrite);
        let mut measured = Measured::new(out);
        let result = self.emit(&mut measured, inline);
        profile::count_add(profile::Counter::TuiBytes, measured.bytes);
        profile::count_add(profile::Counter::TuiWrites, measured.writes);
        result
    }

    fn emit(&mut self, out: &mut impl Write, inline: bool) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        let written = if inline {
            self.write_inline(out)
        } else {
            self.write(out)
        };
        if written.is_err() {
            self.fresh = true;
            self.dirty = true;
        }
        written
    }

    fn write_inline(&mut self, out: &mut impl Write) -> io::Result<()> {
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
        self.stale = None;
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    fn write(&mut self, out: &mut impl Write) -> io::Result<()> {
        let full = self.fresh
            || self.prev.cols() != self.grid.cols()
            || self.prev.rows() != self.grid.rows();
        let tracked = if self.caps.graphics.is_some() || profile::enabled() {
            self.grid.rows()
        } else {
            0
        };
        let rewritten = self.rewritten(tracked, full);
        profile::count_add(
            profile::Counter::TuiRowsWritten,
            rewritten.iter().filter(|row| **row).count() as u64,
        );
        if full {
            self.fresh = false;
            self.stale = None;
            self.prev.mirror(&self.grid);
            crate::terminal::write::all(out, &self.grid, &self.caps)?;
        } else {
            crate::terminal::write::dirty(
                out,
                &self.prev,
                &self.grid,
                &self.caps,
                self.damage.as_ref(),
            )?;
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

    fn rewritten(&self, tracked: usize, full: bool) -> Vec<bool> {
        let differs = |row: usize| {
            !crate::terminal::write::same_row(self.prev.row(row), self.grid.row(row), &self.caps)
        };
        self.damage.as_ref().filter(|_| !full).map_or_else(
            || (0..tracked).map(|row| full || differs(row)).collect(),
            |damage| {
                let mut rewritten = vec![false; tracked];
                for row in damage.rows(tracked) {
                    if let Some(slot) = rewritten.get_mut(row) {
                        *slot = differs(row);
                    }
                }
                rewritten
            },
        )
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
