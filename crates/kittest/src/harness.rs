use std::time::{Duration, Instant};

use accesskit::TreeUpdate;
use sigil::Host;
use sigil::app::{Frame, FrameHooks, FramePaintHook};
use sigil::events::Event;
use sigil::prelude::*;
use sigil::profile::{self, Backend, BackendStatus, FallbackCause};
use sigil::reactive::Runtime;
use sigil::reactive::{__test_clear_clock, __test_set_clock, Owner, next_interval_deadline};
use sigil::tasks::AsyncRuntime;
use sigil::terminal::Screen;

use crate::{Error, ErrorKind};
use gpu_surface::{GPU_STATUS, GpuSurface};

mod actions;
mod gpu_surface;
mod query;
mod snapshot;

pub use actions::Target;
pub use query::{Node, Query};

const FRAME_DT: f32 = 1.0 / 60.0;
const POLL: Duration = Duration::from_millis(5);

type PendingMount = Box<dyn FnOnce(&mut Host)>;

enum HarnessSurface {
    Cells {
        screen: Box<Screen>,
        size: (u16, u16),
    },
    Pixels {
        size: (u32, u32),
        scale: f32,
        fallback: FallbackCause,
    },
    Gpu(Box<GpuSurface>),
}

impl HarnessSurface {
    fn viewport(&self) -> (f32, f32) {
        match self {
            Self::Cells { size, .. } => (
                f32::from(size.0) * Host::CELL,
                f32::from(size.1) * Host::CELL,
            ),
            Self::Pixels { size, .. } => (crate::extent_f32(size.0), crate::extent_f32(size.1)),
            Self::Gpu(gpu) => gpu.viewport,
        }
    }

    const fn backend(&self) -> BackendStatus {
        match self {
            Self::Cells { .. } => BackendStatus::TERMINAL,
            Self::Pixels { fallback, .. } => BackendStatus::new(Backend::Cpu, *fallback),
            Self::Gpu(_) => GPU_STATUS,
        }
    }
}

/// Drives one app the way a user does and checks what it shows.
pub struct Harness {
    clock: Instant,
    owner: Owner,
    pending: Option<PendingMount>,
    host: Host,
    surface: HarnessSurface,
    tasks: AsyncRuntime,
    audited: bool,
}

impl Harness {
    /// Mounts `app` on an 80 by 24 terminal surface; the first step runs the first frame.
    #[must_use]
    pub fn new(app: impl IntoNode + 'static) -> Self {
        let clock = Instant::now();
        __test_set_clock(clock);
        let tasks = AsyncRuntime::new().unwrap_or_else(|err| fail(err));
        tasks.install_current();
        Self {
            clock,
            owner: Owner::new("sigil-kittest-harness", None),
            pending: Some(Box::new(move |host: &mut Host| host.mount(app))),
            host: Host::new(sigil::kernel::HostMode::Cell),
            surface: Self::cells(80, 24),
            tasks,
            audited: true,
        }
    }

    fn cells(cols: u16, rows: u16) -> HarnessSurface {
        HarnessSurface::Cells {
            screen: Box::new(Screen::new(cols, rows)),
            size: (cols, rows),
        }
    }

    /// Uses a terminal surface of `cols` by `rows` cells.
    #[must_use]
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.host = Host::new(sigil::kernel::HostMode::Cell);
        self.surface = Self::cells(cols, rows);
        self
    }

    /// Uses a pixel surface of `width` by `height` logical pixels; `image` renders it and needs a GPU adapter.
    #[must_use]
    pub fn pixels(mut self, width: u32, height: u32) -> Self {
        self.host = Host::new(sigil::kernel::HostMode::Pixel);
        self.surface = HarnessSurface::Pixels {
            size: (width, height),
            scale: 1.0,
            fallback: FallbackCause::None,
        };
        self
    }

    /// Sets the device scale of a pixel or GPU surface; its image is the logical size times this.
    #[must_use]
    #[track_caller]
    pub fn scale(mut self, device_scale: f32) -> Self {
        match &mut self.surface {
            HarnessSurface::Pixels { scale, .. } => *scale = device_scale,
            HarnessSurface::Gpu(gpu) => {
                gpu.rescale(device_scale);
                if self.pending.is_none() {
                    gpu.paint(&mut self.host);
                }
            }
            HarnessSurface::Cells { .. } => {
                fail("scale needs a pixel surface: call pixels or gpu first")
            }
        }
        self
    }

    /// Uses a GPU surface of `width` by `height` logical pixels, or the pixel surface when no adapter exists.
    #[must_use]
    pub fn gpu(mut self, width: u32, height: u32) -> Self {
        self.host = Host::new(sigil::kernel::HostMode::Pixel);
        let opened = GpuSurface::open(width, height);
        if let Some(gpu) = opened.unwrap_or_else(|err| fail(err.chain())) {
            self.surface = HarnessSurface::Gpu(Box::new(gpu));
        } else {
            eprintln!("sigil-kittest: no GPU adapter; gpu surface falls back to pixels");
            self.surface = HarnessSurface::Pixels {
                size: (width, height),
                scale: 1.0,
                fallback: FallbackCause::NoAdapter,
            };
        }
        self
    }

    #[track_caller]
    fn live(&mut self) -> &mut Self {
        if let Some(tree) = self.pending.take() {
            self.tasks.install_current();
            let (owner, host) = (self.owner, &mut self.host);
            owner.with(|| tree(host));
            self.frames(Vec::new())
                .unwrap_or_else(|err| fail(err.chain()));
        }
        self
    }

    fn frame(&mut self, events: Vec<Event>) -> Result<(), Error> {
        self.frame_at(events, FRAME_DT)
    }

    fn frame_at(&mut self, events: Vec<Event>, dt: f32) -> Result<(), Error> {
        self.tasks.install_current();
        let _ = self.tasks.pump();
        let owner = self.owner;
        profile::begin_frame(self.surface.backend());
        let painted = owner.with(|| self.paint(events, dt));
        profile::end_frame();
        painted
    }

    fn paint(&mut self, events: Vec<Event>, dt: f32) -> Result<(), Error> {
        let layout = match &mut self.surface {
            HarnessSurface::Cells { screen, size } => {
                screen.dt(dt);
                screen
                    .frame(&mut self.host, events, *size)
                    .map(|_| ())
                    .and_then(|()| screen.flush(&mut std::io::sink()))
            }
            HarnessSurface::Pixels { size, .. } => {
                let viewport = (crate::extent_f32(size.0), crate::extent_f32(size.1));
                self.host.frame(viewport, dt, hooks(events, None)).layout()
            }
            HarnessSurface::Gpu(gpu) => {
                let viewport = gpu.viewport;
                let paint: FramePaintHook<'_> =
                    Box::new(|host: &mut Host, _: &Frame| gpu.paint(host));
                let layout = self
                    .host
                    .frame(viewport, dt, hooks(events, Some(paint)))
                    .layout();
                gpu.take_error()?;
                layout
            }
        };
        layout.map_err(|err| Error::caused(ErrorKind::Layout, err))
    }

    fn frames(&mut self, events: Vec<Event>) -> Result<(), Error> {
        self.frame(events)?;
        crate::settle::settle(busy, || self.frame_at(Vec::new(), settle_dt()))
    }

    /// Routes `events` in one frame, then runs frames until the UI settles.
    #[track_caller]
    pub fn send(&mut self, events: impl IntoIterator<Item = Event>) -> &mut Self {
        let events = events.into_iter().collect();
        self.live()
            .frames(events)
            .unwrap_or_else(|err| fail(err.chain()));
        self
    }

    /// Runs frames until no signal write and no animation asks for another frame.
    #[track_caller]
    pub fn settle(&mut self) -> &mut Self {
        self.send([])
    }

    /// Moves the test clock forward by `by`, one frame at a time, then settles.
    #[track_caller]
    pub fn advance(&mut self, by: Duration) -> &mut Self {
        self.step(by).settle()
    }

    /// Moves the test clock forward by `by`, one frame at a time, and stops there, so a running animation can be read.
    #[track_caller]
    pub fn step(&mut self, by: Duration) -> &mut Self {
        self.live();
        let step = Duration::from_secs_f32(FRAME_DT);
        let end = self.clock + by;
        while self.clock < end {
            let due = next_interval_deadline().filter(|due| *due > self.clock);
            let next = due.map_or(self.clock + step, |due| due.min(self.clock + step));
            self.clock = next.min(end);
            __test_set_clock(self.clock);
            self.frame(Vec::new())
                .unwrap_or_else(|err| fail(err.chain()));
        }
        self
    }

    #[track_caller]
    fn wait_for<T>(
        &mut self,
        what: &dyn std::fmt::Display,
        mut find: impl FnMut(&mut Self) -> Option<T>,
    ) -> T {
        let start = Instant::now();
        loop {
            if let Some(found) = find(self) {
                return found;
            }
            if start.elapsed() > crate::EXPECT_TIMEOUT {
                let tree = self.accessibility_snapshot();
                fail(format!(
                    "no {what} within {:?}\n{tree}",
                    crate::EXPECT_TIMEOUT
                ));
            }
            std::thread::sleep(POLL);
            self.settle();
        }
    }

    /// The screen text of a terminal surface, or the accessibility snapshot of a pixel surface.
    #[must_use]
    pub fn text(&mut self) -> String {
        if let HarnessSurface::Cells { screen, .. } = &self.live().surface {
            return screen.text();
        }
        self.accessibility_snapshot()
    }

    /// The accessibility tree as indented text, one node per line.
    #[must_use]
    pub fn accessibility_snapshot(&mut self) -> String {
        sigil::a11y::snapshot(&self.accessibility_tree())
    }

    /// The accessibility tree that a screen reader receives.
    #[must_use]
    pub fn accessibility_tree(&mut self) -> TreeUpdate {
        self.live();
        self.host.accessibility_tree()
    }

    /// The host that runs the app, for a check that the steps above cannot make.
    pub fn host(&mut self) -> &mut Host {
        self.live();
        &mut self.host
    }

    /// Lists the design rules that the current frame breaks.
    #[must_use]
    pub fn flaws(&mut self) -> Vec<sigil::layout::audit::Issue> {
        self.live();
        let tolerance = sigil::layout::audit::Tolerance::for_mode(self.host.host_mode());
        sigil::testing::flaws(&self.host, tolerance)
    }

    /// Fails the test if the current frame breaks a design rule.
    #[track_caller]
    pub fn assert_no_flaws(&mut self) {
        let found = self.flaws();
        if found.is_empty() {
            return;
        }
        let listed: Vec<String> = found.iter().map(ToString::to_string).collect();
        let text = self.text();
        fail(format!("design flaws:\n{}\n{text}", listed.join("\n")));
    }

    /// Turns off the design check that a harness runs when it is dropped.
    #[must_use]
    pub const fn unaudited(mut self) -> Self {
        self.audited = false;
        self
    }

    fn viewport(&self) -> (f32, f32) {
        self.surface.viewport()
    }
}

#[track_caller]
#[allow(clippy::panic, reason = "a harness step that fails fails the test")]
pub(crate) fn fail(message: impl std::fmt::Display) -> ! {
    panic!("{message}")
}

fn hooks(events: Vec<Event>, paint: Option<FramePaintHook<'_>>) -> FrameHooks<'_> {
    let hooks = FrameHooks::default().input(move |host: &mut Host| {
        let _effect = sigil::profile::span(sigil::profile::Phase::Effect);
        for event in events {
            sigil::profile::note_input(&event);
            host.route(event);
        }
        None
    });
    match paint {
        Some(paint) => hooks.paint(paint),
        None => hooks,
    }
}

fn busy() -> bool {
    Runtime::frame_dirty() || sigil::animation::active_count() > 0
}

fn settle_dt() -> f32 {
    let perpetual_only =
        sigil::animation::active_count() == 0 && sigil::animation::running_clocks() > 0;
    if perpetual_only { 0.0 } else { FRAME_DT }
}

impl std::fmt::Debug for Harness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Harness")
            .field("viewport", &self.viewport())
            .field("mounted", &self.pending.is_none())
            .field("audited", &self.audited)
            .finish_non_exhaustive()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if self.audited && self.pending.is_none() && !std::thread::panicking() {
            self.assert_no_flaws();
        }
        __test_clear_clock();
        self.owner.dispose();
    }
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
