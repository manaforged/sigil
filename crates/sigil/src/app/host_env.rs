use std::cell::Cell;
use std::rc::Rc;

use crate::geometry::LogicalSize;
use crate::layout::TextMeasure;
use crate::reactive::{Owner, Signal, owner_by_id, signal};

thread_local! {
    static VIEWPORT: Cell<Option<Signal<LogicalSize>>> = const { Cell::new(None) };
    static DETACHED: Cell<Option<Signal<LogicalSize>>> = const { Cell::new(None) };
    static CAPS: Cell<HostCaps> = const { Cell::new(HostCaps::window()) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HostCaps {
    pub(crate) hyperlinks: bool,
}

impl HostCaps {
    #[must_use]
    pub(crate) fn current() -> Self {
        CAPS.with(Cell::get)
    }

    #[must_use]
    pub(crate) const fn window() -> Self {
        Self { hyperlinks: true }
    }
}

impl Default for HostCaps {
    fn default() -> Self {
        Self::window()
    }
}

/// The viewport size in logical px.
#[must_use]
#[track_caller]
pub fn viewport_size() -> Signal<LogicalSize> {
    VIEWPORT.with(Cell::get).unwrap_or_else(|| {
        detached(
            &DETACHED,
            "sigil::detached_viewport",
            LogicalSize::default(),
        )
    })
}

fn detached<T: 'static>(
    slot: &'static std::thread::LocalKey<Cell<Option<Signal<T>>>>,
    name: &'static str,
    initial: T,
) -> Signal<T> {
    slot.with(|slot| {
        slot.get().unwrap_or_else(|| {
            let value = Owner::new(name, None).with(|| signal(initial));
            slot.set(Some(value));
            value
        })
    })
}

/// `true` on a cell grid (terminal) host.
#[must_use]
pub fn is_grid() -> bool {
    crate::style::cell_size().is_some()
}

pub(super) struct HostEnv {
    cell: f32,
    geometry: Rc<dyn TextMeasure>,
    pub(super) exit: Rc<Cell<bool>>,
    focus_source: Rc<Cell<bool>>,
    viewport: Signal<LogicalSize>,
    pub(super) caps: HostCaps,
    pub(super) theme: Signal<crate::style::Theme>,
}

pub(crate) struct EnvGuard {
    cell: f32,
    geometry: Option<Rc<dyn TextMeasure>>,
    exit: Option<Rc<Cell<bool>>>,
    focus_source: Option<Rc<Cell<bool>>>,
    viewport: Option<Signal<LogicalSize>>,
    caps: HostCaps,
    theme: Option<Signal<crate::style::Theme>>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        crate::style::swap_cell(self.cell);
        let _ = crate::layout::text_geometry::swap_geometry(self.geometry.take());
        let _ = crate::app::exit::swap_target(self.exit.take());
        let _ = crate::events::focus::swap_focus_source(self.focus_source.take());
        VIEWPORT.with(|slot| slot.set(self.viewport.take()));
        CAPS.with(|slot| slot.set(self.caps));
        let _ = crate::style::theme::ambient::swap_active_theme(self.theme.take());
    }
}

impl HostEnv {
    pub(super) fn new(cell: f32, geometry: Rc<dyn TextMeasure>, owner: Owner) -> Self {
        let mut signals = None;
        owner_by_id(owner, || {
            signals = Some((
                signal(LogicalSize::default()),
                signal(crate::style::Theme::default()),
            ));
        });
        let (viewport, theme) = signals.expect("host root owner is live");
        Self {
            cell,
            geometry,
            exit: Rc::new(Cell::new(false)),
            focus_source: Rc::new(Cell::new(true)),
            viewport,
            caps: HostCaps::window(),
            theme,
        }
    }

    pub(super) fn enter(&self) -> EnvGuard {
        EnvGuard {
            cell: crate::style::swap_cell(self.cell),
            geometry: crate::layout::text_geometry::swap_geometry(Some(Rc::clone(&self.geometry))),
            exit: crate::app::exit::swap_target(Some(Rc::clone(&self.exit))),
            focus_source: crate::events::focus::swap_focus_source(Some(Rc::clone(
                &self.focus_source,
            ))),
            viewport: VIEWPORT.with(|slot| slot.replace(Some(self.viewport))),
            caps: CAPS.with(|slot| slot.replace(self.caps)),
            theme: crate::style::theme::ambient::swap_active_theme(Some(self.theme)),
        }
    }

    pub(super) fn publish_viewport(&self, viewport: (f32, f32)) {
        self.viewport.set(LogicalSize::new(viewport.0, viewport.1));
    }
}
