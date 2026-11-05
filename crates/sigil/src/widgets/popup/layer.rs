use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::events::Key;
use crate::kernel::{FocusScopeKind, Live, NodeBuilder, TopLayer, announce};
use crate::layout::Display;
use crate::layout::{Popup, ViewportOverlay};
use crate::reactive::{Signal, effect_named, on_cleanup};
use crate::style::StyleAtom;
use crate::widgets::Widget;

#[derive(Clone, Copy)]
pub(crate) enum PopupAnchor {
    Inline,
    Viewport,
    Top,
    Popup(Popup),
}

#[derive(Clone, Copy)]
pub(crate) struct PopupLayer {
    pub(crate) show: Signal<bool>,
    pub(crate) anchor: PopupAnchor,
    pub(crate) trap: bool,
}

impl PopupLayer {
    pub(crate) const fn new(show: Signal<bool>, anchor: PopupAnchor) -> Self {
        Self {
            show,
            anchor,
            trap: false,
        }
    }

    pub(crate) const fn trap(mut self, trap: bool) -> Self {
        self.trap = trap;
        self
    }

    pub(crate) fn wrap(self, node: NodeBuilder) -> NodeBuilder {
        let show = self.show;
        let node = match self.anchor {
            PopupAnchor::Inline => node,
            PopupAnchor::Viewport => node.with(ViewportOverlay),
            PopupAnchor::Top => node.with(TopLayer).with(ViewportOverlay),
            PopupAnchor::Popup(popup) => node.with(TopLayer).with(ViewportOverlay).with(popup),
        };
        let node = node.style_reactive(move || shown(show));
        if self.trap {
            node.mutate(|node| node.a11y.focus_scope = Some(FocusScopeKind::Modal))
        } else {
            node
        }
    }

    pub(crate) fn dismiss_outside(self, node: NodeBuilder) -> NodeBuilder {
        let show = self.show;
        node.pointer_down(move |event| {
            if hide(show) {
                event.stop_propagation();
            }
        })
    }

    pub(crate) fn register_escape(self) {
        register_escape(self.show);
    }

    /// Closes this popup when a press lands outside every popup panel.
    pub(crate) fn register_light(self) {
        let id = NEXT_ESCAPE_LAYER_ID.fetch_add(1, Ordering::Relaxed);
        let show = self.show;
        LIGHT_LAYERS.with(|stack| stack.borrow_mut().push(EscapeLayer { id, show }));
        on_cleanup(move || {
            LIGHT_LAYERS.with(|stack| stack.borrow_mut().retain(|layer| layer.id != id));
        });
    }

    pub(crate) fn announce_open(self, name: &'static str, open: impl Fn() -> String + 'static) {
        let show = self.show;
        let prev = Rc::new(Cell::new(false));
        effect_named(name, move || {
            let open_now = show.get();
            let was_open = prev.replace(open_now);
            if open_now && !was_open {
                announce(open(), Live::Polite);
            }
        });
    }
}

pub(crate) fn close_on_escape(node: NodeBuilder) -> NodeBuilder {
    node.key(Key::Escape, |event| {
        if close_top_escape() {
            event.stop_propagation();
        }
    })
}

pub(crate) fn hide(show: Signal<bool>) -> bool {
    let shown = show.peek();
    if shown {
        show.set(false);
    }
    shown
}

pub(crate) fn shown(show: Signal<bool>) -> &'static [StyleAtom] {
    shown_when(show.get())
}

pub(crate) const fn shown_when(show: bool) -> &'static [StyleAtom] {
    if show {
        &[]
    } else {
        &[StyleAtom::Display(Display::None)]
    }
}

#[derive(Clone, Copy)]
struct EscapeLayer {
    id: u64,
    show: Signal<bool>,
}

thread_local! {
    static ESCAPE_LAYERS: RefCell<Vec<EscapeLayer>> = const {
        RefCell::new(Vec::new())
    };
    static LIGHT_LAYERS: RefCell<Vec<EscapeLayer>> = const {
        RefCell::new(Vec::new())
    };
}

/// Closes every open light popup; false when none was open.
pub(crate) fn close_light() -> bool {
    let open: Vec<EscapeLayer> = LIGHT_LAYERS.with(|stack| stack.borrow().iter().copied().filter(|layer| layer.show.peek()).collect());
    open.iter().fold(false, |closed, layer| hide(layer.show) || closed)
}

static NEXT_ESCAPE_LAYER_ID: AtomicU64 = AtomicU64::new(1);

fn register_escape(show: Signal<bool>) {
    let id = NEXT_ESCAPE_LAYER_ID.fetch_add(1, Ordering::Relaxed);
    ESCAPE_LAYERS.with(|stack| stack.borrow_mut().push(EscapeLayer { id, show }));
    on_cleanup(move || {
        ESCAPE_LAYERS.with(|stack| {
            stack.borrow_mut().retain(|layer| layer.id != id);
        });
    });
}

/// Closes the topmost open popup; false when none is open.
pub fn close_top_escape() -> bool {
    ESCAPE_LAYERS.with(|stack| {
        let top = stack
            .borrow()
            .iter()
            .rev()
            .copied()
            .find(|layer| layer.show.peek());
        top.is_some_and(|layer| hide(layer.show))
    })
}
