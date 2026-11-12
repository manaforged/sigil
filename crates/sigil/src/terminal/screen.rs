use crate::Error;
use crate::Host;
use crate::events::Event;
use crate::profile;
use crate::scene::CommandScene;

use crate::terminal::access::{Access, Announcer, Place, Speech};
use crate::terminal::caps::Capabilities;
use crate::terminal::paint::{Grid, Marks};
use crate::terminal::run::view;
use crate::terminal::transcript::Transcript;
use crate::terminal::write::{CountingWriter, RowDamage};

mod flush;
mod raster;

const CELL: f32 = Host::CELL;

/// The terminal screen state between frames.
#[allow(
    clippy::struct_excessive_bools,
    reason = "four independent facts about the last frame"
)]
pub struct Screen {
    scene: CommandScene,
    grid: Grid,
    prev: Grid,
    fresh: bool,
    blank: bool,
    dirty: bool,
    exited: bool,
    dt: f32,
    painter: crate::terminal::graphics::ImagePainter,
    transcript: Transcript,
    caps: Capabilities,
    live_rows: usize,
    announcer: Announcer,
    damage: Option<RowDamage>,
    marks: Marks,
    stale: Option<Vec<usize>>,
    projection: Projection,
}

impl core::fmt::Debug for Screen {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Screen")
            .field("fresh", &self.fresh)
            .field("dirty", &self.dirty)
            .field("exited", &self.exited)
            .field("dt", &self.dt)
            .field("caps", &self.caps)
            .field("live_rows", &self.live_rows)
            .finish_non_exhaustive()
    }
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
    /// Creates a screen.
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            scene: CommandScene::default(),
            grid: Grid::new(cols, rows),
            blank: true,
            prev: Grid::new(0, 0),
            fresh: true,
            dirty: true,
            exited: false,
            dt: crate::app::FRAME_INTERVAL.as_secs_f32(),
            painter: crate::terminal::graphics::ImagePainter::default(),
            transcript: Transcript::from_env(),
            caps: Capabilities::default(),
            live_rows: 0,
            announcer: Announcer::default(),
            damage: None,
            marks: Marks::default(),
            stale: None,
            projection: Projection::default(),
        }
    }

    kittest_pub! {
        /// Creates a screen with these capabilities.
        #[doc(hidden)]
        #[must_use]
        pub fn with_caps(cols: u16, rows: u16, caps: Capabilities) -> Self {
            let mut screen = Self::new(cols, rows);
            screen.set_caps(caps);
            screen
        }
    }

    kittest_pub! {
        /// Sets the caps.
        #[doc(hidden)]
        pub const fn set_caps(&mut self, caps: Capabilities) {
            self.caps = caps;
            self.grid.set_graphics(caps.graphics.is_some());
            self.prev.set_graphics(caps.graphics.is_some());
            self.fresh = true;
        }
    }

    /// Sets the access.
    pub const fn set_access(&mut self, access: Access) {
        self.announcer.access = access;
        self.projection.last = Projected::Never;
    }

    pub(crate) const fn keep_pending(&mut self) {
        self.fresh = true;
        if !self.dirty {
            core::mem::swap(&mut self.prev, &mut self.grid);
            self.dirty = true;
        }
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

    /// Returns the caps.
    #[must_use]
    pub const fn caps(&self) -> &Capabilities {
        &self.caps
    }

    /// Runs and paints one frame.
    pub fn frame(
        &mut self,
        host: &mut Host,
        events: impl IntoIterator<Item = Event>,
        size: (u16, u16),
    ) -> Result<&Grid, Error> {
        host.drop_cell_screen();
        self.frame_inner(host, events, size)
    }

    pub(crate) fn grid_frame(
        &mut self,
        host: &mut Host,
        events: impl IntoIterator<Item = Event>,
        size: (u16, u16),
    ) -> Result<Grid, Error> {
        let out = self.frame_inner(host, events, size)?.clone();
        if self.dirty {
            if self.fresh
                || self.prev.cols() != self.grid.cols()
                || self.prev.rows() != self.grid.rows()
            {
                self.fresh = false;
                self.stale = None;
                self.prev.fit(&self.grid);
            }
            self.dirty = false;
            core::mem::swap(&mut self.prev, &mut self.grid);
        }
        Ok(out)
    }

    fn frame_inner(
        &mut self,
        host: &mut Host,
        events: impl IntoIterator<Item = Event>,
        size: (u16, u16),
    ) -> Result<&Grid, Error> {
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
        let out = host.frame(view(cols, rows), dt, hooks);
        self.exited |= out.exit;
        if crate::hud::enabled() {
            crate::terminal::hud::draw(&mut self.grid);
            self.damage = None;
            self.stale = None;
            if !self.dirty && !self.fresh && !self.unchanged() {
                self.dirty = true;
            }
        }
        let update = self.read_tree(host);
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

    fn read_tree(&mut self, host: &Host) -> Option<accesskit::TreeUpdate> {
        let records = self.transcript.records();
        let skip = self.projection.quiet
            && self.projection.last == Projected::Focus(host.focus())
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
        let update = host.accessibility_tree_scoped(scope);
        self.projection.last = Projected::Focus(host.focus());
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
            self.damage = Some(RowDamage::new(Vec::new(), false));
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

    /// Returns whether exited is set.
    #[must_use]
    pub const fn exited(&self) -> bool {
        self.exited
    }

    /// Returns the text.
    #[must_use]
    pub fn text(&self) -> String {
        if self.dirty { &self.grid } else { &self.prev }.text()
    }

    /// Returns the rows.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.grid.rows()
    }

    /// Sets the time step of the next frame.
    pub const fn set_dt(&mut self, dt: f32) {
        self.dt = dt;
    }

    /// Repaints every cell on the next flush.
    pub const fn invalidate(&mut self) {
        self.fresh = true;
    }

    /// Returns whether needs flush is set.
    #[must_use]
    pub const fn needs_flush(&self) -> bool {
        self.dirty
    }
}
