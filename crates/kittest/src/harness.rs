use std::time::{Duration, Instant};

use sigil::app::{AccessibilityTree, FrameHooks, TreeRoot};
use sigil::prelude::*;
use sigil::reactive::{__test_clear_clock, __test_set_clock, Owner, next_interval_deadline};
use sigil::terminal::Screen;

use crate::Error;

mod actions;
mod query;
mod snapshot;

pub use actions::Target;
pub use query::{Node, Query};
pub use snapshot::SNAPSHOT_MAX_DELTA_PCT;

const FRAME_DT: f32 = 1.0 / 60.0;

type Pending = Box<dyn FnOnce(&mut Cx) -> NodeId>;

enum Surface {
    Cells {
        screen: Box<Screen>,
        size: (u16, u16),
    },
    Pixels {
        size: (u32, u32),
    },
}

impl Surface {
    fn viewport(&self) -> (f32, f32) {
        match self {
            Self::Cells { size, .. } => (
                f32::from(size.0) * Host::CELL,
                f32::from(size.1) * Host::CELL,
            ),
            Self::Pixels { size } => (crate::extent_f32(size.0), crate::extent_f32(size.1)),
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
}

impl Harness {
    #[must_use]
    pub fn new(tree: impl TreeRoot + 'static) -> Self {
        let clock = Instant::now();
        __test_set_clock(clock);
        Self {
            clock,
            owner: Owner::new("sigil-kittest-harness", None),
            pending: Some(Box::new(move |cx| tree.install(cx))),
            host: Host::cells(),
            router: EventRouter::new(),
            surface: Self::cells(80, 24),
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
        };
        self
    }

    #[track_caller]
    fn live(&mut self) -> &mut Self {
        if let Some(tree) = self.pending.take() {
            let (owner, host) = (self.owner, &mut self.host);
            owner.with(|| host.build(tree));
            self.frames(Vec::new()).unwrap_or_else(|err| fail(err));
        }
        self
    }

    fn frame(&mut self, events: Vec<ShellEvent>) -> Result<(), Error> {
        let owner = self.owner;
        owner.with(|| self.paint(events))
    }

    fn paint(&mut self, events: Vec<ShellEvent>) -> Result<(), Error> {
        let layout = match &mut self.surface {
            Surface::Cells { screen, size } => screen
                .frame(&mut self.host, &mut self.router, events, *size)
                .map(|_| ()),
            Surface::Pixels { size } => {
                let viewport = (crate::extent_f32(size.0), crate::extent_f32(size.1));
                let hooks = FrameHooks {
                    input: Some(Box::new(
                        move |host: &mut Host, router: &mut EventRouter| {
                            for event in events {
                                host.route(router, event);
                            }
                        },
                    )),
                    ..FrameHooks::default()
                };
                self.host
                    .frame(&mut self.router, viewport, FRAME_DT, hooks)
                    .layout
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
