use std::time::{Duration, Instant};

use sigil::Host;
use sigil::app::{AccessibilityTree, FrameHooks, FrameInfo, FramePaintHook};
use sigil::events::EventRouter;
use sigil::events::ShellEvent;
use sigil::prelude::*;
use sigil::profile::{self, BackendKind, BackendStatus, FallbackCause};
use sigil::reactive::Runtime;
use sigil::reactive::{__test_clear_clock, __test_set_clock, Owner, next_interval_deadline};
use sigil::tasks::AsyncRuntime;
use sigil::terminal::Screen;

use crate::Error;
use gpu_surface::{GPU_STATUS, GpuSurface};

mod actions;
mod gpu_surface;
mod query;
mod snapshot;

pub use actions::Target;
pub use query::{Node, Query};
pub use snapshot::SNAPSHOT_MAX_DELTA_PCT;

const FRAME_DT: f32 = 1.0 / 60.0;
const POLL: Duration = Duration::from_millis(5);

type Pending = Box<dyn FnOnce(&mut Host)>;

enum Surface {
    Cells {
        screen: Box<Screen>,
        size: (u16, u16),
    },
    Pixels {
        size: (u32, u32),
        fallback: FallbackCause,
    },
    Gpu(Box<GpuSurface>),
}

impl Surface {
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
            Self::Pixels { fallback, .. } => BackendStatus {
                kind: BackendKind::Cpu,
                fallback: *fallback,
            },
            Self::Gpu(_) => GPU_STATUS,
        }
    }
}

pub struct Harness {
    clock: Instant,
    owner: Owner,
    pending: Option<Pending>,
    host: Host,
    router: EventRouter,
    surface: Surface,
    tasks: AsyncRuntime,
}

impl Harness {
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
            host: Host::cells(),
            router: EventRouter::new(),
            surface: Self::cells(80, 24),
            tasks,
        }
    }

    fn cells(cols: u16, rows: u16) -> Surface {
        Surface::Cells {
            screen: Box::new(Screen::new(cols, rows)),
            size: (cols, rows),
        }
    }

    #[must_use]
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.host = Host::cells();
        self.surface = Self::cells(cols, rows);
        self
    }

    #[must_use]
    pub fn pixels(mut self, width: u32, height: u32) -> Self {
        self.host = Host::new();
        self.surface = Surface::Pixels {
            size: (width, height),
            fallback: FallbackCause::None,
        };
        self
    }

    #[must_use]
    pub fn gpu(mut self, width: u32, height: u32, scale: f32) -> Self {
        self.host = Host::new();
        if let Some(gpu) = GpuSurface::open(width, height, scale).unwrap_or_else(|err| fail(err)) {
            self.surface = Surface::Gpu(Box::new(gpu));
        } else {
            eprintln!("sigil-kittest: no GPU adapter; gpu surface falls back to pixels");
            self.surface = Surface::Pixels {
                size: (width, height),
                fallback: FallbackCause::NoAdapter,
            };
        }
        self
    }

    #[track_caller]
    fn live(&mut self) -> &mut Self {
        if let Some(tree) = self.pending.take() {
            let (owner, host) = (self.owner, &mut self.host);
            owner.with(|| tree(host));
            self.frames(Vec::new()).unwrap_or_else(|err| fail(err));
        }
        self
    }

    fn frame(&mut self, events: Vec<ShellEvent>) -> Result<(), Error> {
        let _ = self.tasks.pump();
        let owner = self.owner;
        profile::begin_frame(self.surface.backend());
        let painted = owner.with(|| self.paint(events));
        profile::tick_frame();
        painted
    }

    fn paint(&mut self, events: Vec<ShellEvent>) -> Result<(), Error> {
        let layout = match &mut self.surface {
            Surface::Cells { screen, size } => screen
                .frame(&mut self.host, &mut self.router, events, *size)
                .map(|_| ()),
            Surface::Pixels { size, .. } => {
                let viewport = (crate::extent_f32(size.0), crate::extent_f32(size.1));
                self.host
                    .frame(&mut self.router, viewport, FRAME_DT, hooks(events, None))
                    .layout
            }
            Surface::Gpu(gpu) => {
                let viewport = gpu.viewport;
                let paint: FramePaintHook<'_> =
                    Box::new(|host: &mut Host, _: &mut EventRouter, _: &FrameInfo| gpu.paint(host));
                let layout = self
                    .host
                    .frame(
                        &mut self.router,
                        viewport,
                        FRAME_DT,
                        hooks(events, Some(paint)),
                    )
                    .layout;
                gpu.take_error()?;
                layout
            }
        };
        layout.map_err(|err| Error::Layout(err.to_string()))
    }

    fn frames(&mut self, events: Vec<ShellEvent>) -> Result<(), Error> {
        self.frame(events)?;
        crate::settle::settle(busy, || self.frame(Vec::new()))
    }

    #[track_caller]
    pub fn send(&mut self, events: impl IntoIterator<Item = ShellEvent>) -> &mut Self {
        let events = events.into_iter().collect();
        self.live().frames(events).unwrap_or_else(|err| fail(err));
        self
    }

    #[track_caller]
    pub fn settle(&mut self) -> &mut Self {
        self.send([])
    }

    #[track_caller]
    pub fn advance(&mut self, by: Duration) -> &mut Self {
        self.live();
        let step = Duration::from_secs_f32(FRAME_DT);
        let end = self.clock + by;
        while self.clock < end {
            let due = next_interval_deadline().filter(|due| *due > self.clock);
            let next = due.map_or(self.clock + step, |due| due.min(self.clock + step));
            self.clock = next.min(end);
            __test_set_clock(self.clock);
            self.frame(Vec::new()).unwrap_or_else(|err| fail(err));
        }
        self.settle()
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

    #[must_use]
    pub fn text(&mut self) -> String {
        if let Surface::Cells { screen, .. } = &self.live().surface {
            return screen.text();
        }
        self.accessibility_snapshot()
    }

    #[must_use]
    pub fn accessibility_snapshot(&mut self) -> String {
        sigil::a11y::snapshot(&self.accessibility_tree())
    }

    #[must_use]
    pub fn accessibility_tree(&mut self) -> AccessibilityTree {
        self.live();
        self.host.accessibility_tree(&self.router)
    }

    pub fn host(&mut self) -> (&mut Host, &mut EventRouter) {
        self.live();
        (&mut self.host, &mut self.router)
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

fn hooks(events: Vec<ShellEvent>, paint: Option<FramePaintHook<'_>>) -> FrameHooks<'_> {
    FrameHooks {
        input: Some(Box::new(
            move |host: &mut Host, router: &mut EventRouter| {
                let _effect = sigil::profile::span(sigil::profile::Phase::Effect);
                for event in events {
                    sigil::profile::note_input(&event);
                    host.route(router, event);
                }
                None
            },
        )),
        paint,
        ..FrameHooks::default()
    }
}

fn busy() -> bool {
    Runtime::frame_dirty() || sigil::animation_runtime::active_count() > 0
}

impl Drop for Harness {
    fn drop(&mut self) {
        __test_clear_clock();
        self.owner.dispose();
    }
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod tests;
