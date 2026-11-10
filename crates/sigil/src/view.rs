use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use crate::error::{Error, ErrorKind};
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container, fragment, show};
use crate::layout::{LayoutStyle, Space, column};
use crate::reactive::{Signal, derived, interval, signal};
use crate::style::Tone;
use crate::widgets::{Widget, notice_band};

mod build;
mod lex;
mod parse;
mod vocab;

use parse::{Action, Item, Slot, parse};

const POLL: Duration = Duration::from_millis(200);

#[derive(Default)]
struct Binds {
    signals: HashMap<String, Signal<String>>,
    flags: HashMap<String, Signal<bool>>,
    actions: HashMap<String, Action>,
    slots: HashMap<String, Slot>,
}

#[doc = include_str!("../docs/view.md")]
#[must_use]
pub struct View {
    source: String,
    path: Option<PathBuf>,
    binds: Binds,
}

impl core::fmt::Debug for View {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("View")
            .field("source", &self.source)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Builds a `View` from a file relative to `Cargo.toml`, watched in debug builds.
#[macro_export]
macro_rules! view {
    ($path:literal) => {{
        let view = $crate::View::new(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/",
            $path
        )));
        #[cfg(debug_assertions)]
        let view = view.watch(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path));
        view
    }};
}

impl View {
    /// Creates a view from `.sigil` text.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            path: None,
            binds: Binds::default(),
        }
    }

    /// Reloads the view when the file at `path` changes.
    pub fn watch(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Binds `$name` to a text signal.
    pub fn signal(mut self, name: &str, value: Signal<String>) -> Self {
        self.binds.signals.insert(name.to_owned(), value);
        self
    }

    /// Binds `$name` to a true or false signal.
    pub fn flag(mut self, name: &str, value: Signal<bool>) -> Self {
        self.binds.flags.insert(name.to_owned(), value);
        self
    }

    /// Binds `@name` to a function.
    pub fn action(mut self, name: &str, action: impl Fn() + 'static) -> Self {
        self.binds.actions.insert(name.to_owned(), Rc::new(action));
        self
    }

    /// Binds `slot name` to a function that builds Rust widgets.
    pub fn slot<N>(mut self, name: &str, slot: impl Fn() -> N + 'static) -> Self
    where
        N: IntoNode + 'static,
    {
        let slot: Slot = Rc::new(move || container().child(slot()));
        self.binds.slots.insert(name.to_owned(), slot);
        self
    }

    /// Returns the first problem in the text and its line and column.
    pub fn check(&self) -> Result<(), Error> {
        parse(&self.source, &self.binds)
            .map(|_| ())
            .map_err(|err| Error::message(ErrorKind::Config, err))
    }

    fn mount(self) -> NodeBuilder {
        let binds = Rc::new(self.binds);
        let (items, problem) = match parse(&self.source, &binds) {
            Ok(items) => (items, None),
            Err(err) => (Vec::new(), Some(err.to_string())),
        };
        let items = signal(Rc::new(items));
        let problem = signal(problem);
        if let Some(path) = self.path {
            let watch = Watch {
                path,
                seen: RefCell::new((None, self.source)),
                binds,
                items,
                problem,
            };
            interval(POLL, move || watch.poll());
        }
        let failed = derived(move || problem.with(Option::is_some));
        let band = move || notice_band(move || problem.get().unwrap_or_default(), Tone::Error);
        column()
            .layout(LayoutStyle::default().fill_width().grow(1.0))
            .gap(Space::None)
            .child(show(failed, band, fragment))
            .child(column().gap(Space::None).children_reactive(move || {
                items.get().iter().map(build::build).collect::<Vec<_>>()
            }))
    }
}

struct Watch {
    path: PathBuf,
    seen: RefCell<(Option<SystemTime>, String)>,
    binds: Rc<Binds>,
    items: Signal<Rc<Vec<Rc<Item>>>>,
    problem: Signal<Option<String>>,
}

impl Watch {
    fn poll(&self) {
        let Ok(changed) = std::fs::metadata(&self.path).and_then(|file| file.modified()) else {
            return;
        };
        let mut seen = self.seen.borrow_mut();
        if seen.0 == Some(changed) {
            return;
        }
        seen.0 = Some(changed);
        let Ok(source) = std::fs::read_to_string(&self.path) else {
            return;
        };
        if source == seen.1 {
            return;
        }
        match parse(&source, &self.binds) {
            Ok(items) => {
                self.items.update(|shown| *shown = Rc::new(items));
                self.problem.set(None);
            }
            Err(err) => {
                tracing::warn!(target: "sigil::view", "{}: {err}", self.path.display());
                self.problem.set(Some(err.to_string()));
            }
        }
        seen.1 = source;
    }
}

impl IntoNode for View {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.mount().build(cx)
    }
}
