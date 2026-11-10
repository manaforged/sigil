use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::geometry::LogicalSize;
use crate::layout::TextMeasure;
use crate::reactive::{Owner, Signal, owner_by_id, signal};

thread_local! {
    static VIEWPORT: Cell<Option<Signal<LogicalSize>>> = const { Cell::new(None) };
    static DETACHED: Cell<Option<Signal<LogicalSize>>> = const { Cell::new(None) };
    static CAPS: Cell<HostCaps> = const { Cell::new(HostCaps::window()) };
    static ENTERED: Cell<bool> = const { Cell::new(false) };
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

type Swap<T> = fn(T) -> T;

fn swap<T: Default>(field: &mut T, swap: Swap<T>) {
    *field = swap(core::mem::take(field));
}

fn adopt<T: Default>(field: &mut T, swap: Swap<T>, merge: fn(&mut T, T)) {
    merge(field, swap(T::default()));
}

#[derive(Default)]
struct Scoped {
    cell: f32,
    geometry: Option<Rc<dyn TextMeasure>>,
    exit: Option<Rc<Cell<bool>>>,
    focus_source: Option<Rc<Cell<bool>>>,
    viewport: Option<Signal<LogicalSize>>,
    theme: Option<Signal<crate::style::Theme>>,
    layers: crate::widgets::PopupStacks,
    focus_requests: Vec<crate::kernel::NodeId>,
    announcements: std::collections::VecDeque<(String, crate::kernel::Live)>,
    pending_theme: Option<crate::style::Theme>,
    #[cfg(feature = "dialog")]
    dialogs: Vec<super::dialog::layer::Request>,
}

impl Scoped {
    fn exchange(&mut self) {
        swap(&mut self.cell, crate::style::swap_cell);
        swap(
            &mut self.geometry,
            crate::layout::text_geometry::swap_geometry,
        );
        swap(&mut self.exit, crate::app::exit::swap_target);
        swap(
            &mut self.focus_source,
            crate::events::focus::swap_focus_source,
        );
        swap(&mut self.viewport, |value| VIEWPORT.replace(value));
        swap(
            &mut self.theme,
            crate::style::theme::ambient::swap_active_theme,
        );
        swap(&mut self.layers, crate::widgets::popup_swap_layers);
        swap(
            &mut self.focus_requests,
            crate::events::focus::swap_focus_requests,
        );
        swap(
            &mut self.announcements,
            crate::kernel::Cx::swap_announcements,
        );
        swap(&mut self.pending_theme, super::appearance::swap_pending);
        #[cfg(feature = "dialog")]
        swap(&mut self.dialogs, super::dialog::layer::swap_queue);
    }

    fn adopt_outer(&mut self) {
        adopt(
            &mut self.focus_requests,
            crate::events::focus::swap_focus_requests,
            Extend::extend,
        );
        adopt(
            &mut self.announcements,
            crate::kernel::Cx::swap_announcements,
            Extend::extend,
        );
        adopt(
            &mut self.pending_theme,
            super::appearance::swap_pending,
            |field, outer| {
                *field = outer.or_else(|| field.take());
            },
        );
        #[cfg(feature = "dialog")]
        adopt(
            &mut self.dialogs,
            super::dialog::layer::swap_queue,
            Extend::extend,
        );
    }
}

pub(super) struct HostEnv {
    pub(super) exit: Rc<Cell<bool>>,
    viewport: Signal<LogicalSize>,
    pub(super) caps: HostCaps,
    pub(super) theme: Signal<crate::style::Theme>,
    scoped: Rc<RefCell<Option<Scoped>>>,
}

pub(crate) struct EnvGuard {
    home: Option<Home>,
}

struct Home {
    slot: Rc<RefCell<Option<Scoped>>>,
    scoped: Scoped,
    caps: HostCaps,
    nested: bool,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(mut home) = self.home.take() {
            home.scoped.exchange();
            swap(&mut home.caps, |value| CAPS.replace(value));
            ENTERED.set(home.nested);
            *home.slot.borrow_mut() = Some(home.scoped);
        }
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
        let exit = Rc::new(Cell::new(false));
        let scoped = Scoped {
            cell,
            geometry: Some(geometry),
            exit: Some(Rc::clone(&exit)),
            focus_source: Some(Rc::new(Cell::new(true))),
            viewport: Some(viewport),
            theme: Some(theme),
            ..Scoped::default()
        };
        Self {
            exit,
            viewport,
            caps: HostCaps::window(),
            theme,
            scoped: Rc::new(RefCell::new(Some(scoped))),
        }
    }

    pub(super) fn enter(&self) -> EnvGuard {
        let Some(mut scoped) = self.scoped.borrow_mut().take() else {
            return EnvGuard { home: None };
        };
        let nested = ENTERED.replace(true);
        if !nested {
            scoped.adopt_outer();
        }
        scoped.exchange();
        let mut caps = self.caps;
        swap(&mut caps, |value| CAPS.replace(value));
        EnvGuard {
            home: Some(Home {
                slot: Rc::clone(&self.scoped),
                scoped,
                caps,
                nested,
            }),
        }
    }

    pub(super) fn publish_viewport(&self, viewport: (f32, f32)) {
        self.viewport.set(LogicalSize::new(viewport.0, viewport.1));
    }
}
