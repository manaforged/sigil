use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::LocalKey;

use crate::events::NamedKey;
use crate::kernel::{FocusScopeKind, Live, NodeBuilder, TopLayer, announce};
use crate::layout::Display;
use crate::layout::{Popup, ViewportOverlay};
use crate::reactive::{Signal, cleanup, effect_named, signal};
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
        register_layer(&ESCAPE_LAYERS, self.show);
    }

    /// Closes this popup when a press lands outside every popup panel.
    pub(crate) fn register_light(self) {
        register_layer(&LIGHT_LAYERS, self.show);
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
    node.key(NamedKey::Escape, |event| {
        if close_popup() {
            event.prevent_default();
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
pub(crate) struct EscapeLayer {
    id: u64,
    show: Signal<bool>,
    opened_at: Signal<u64>,
}

thread_local! {
    static ESCAPE_LAYERS: RefCell<Vec<EscapeLayer>> = const {
        RefCell::new(Vec::new())
    };
    static LIGHT_LAYERS: RefCell<Vec<EscapeLayer>> = const {
        RefCell::new(Vec::new())
    };
}

pub(crate) type Stacks = (Vec<EscapeLayer>, Vec<EscapeLayer>);

pub(crate) fn swap_layers(stacks: Stacks) -> Stacks {
    (
        ESCAPE_LAYERS.replace(stacks.0),
        LIGHT_LAYERS.replace(stacks.1),
    )
}

/// Closes every open light popup in reverse open order; false when none was open.
pub(crate) fn close_light() -> bool {
    let mut open: Vec<EscapeLayer> = LIGHT_LAYERS.with(|stack| {
        stack
            .borrow()
            .iter()
            .copied()
            .filter(|layer| layer.show.peek())
            .collect()
    });
    open.sort_by_key(|layer| std::cmp::Reverse((layer.opened_at.peek(), layer.id)));
    let mut closed = false;
    for layer in &open {
        closed |= hide(layer.show);
    }
    closed
}

static NEXT_ESCAPE_LAYER_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_OPEN_SEQ: AtomicU64 = AtomicU64::new(1);

fn register_layer(stack: &'static LocalKey<RefCell<Vec<EscapeLayer>>>, show: Signal<bool>) {
    let id = NEXT_ESCAPE_LAYER_ID.fetch_add(1, Ordering::Relaxed);
    let initial = if show.peek() {
        NEXT_OPEN_SEQ.fetch_add(1, Ordering::Relaxed)
    } else {
        0
    };
    let opened_at = signal(initial);
    let was_open = Cell::new(show.peek());
    effect_named("sigil::popup::open_order", move || {
        let open_now = show.get();
        if open_now && !was_open.replace(open_now) {
            opened_at.set(NEXT_OPEN_SEQ.fetch_add(1, Ordering::Relaxed));
        }
    });
    stack.with(|layers| {
        layers.borrow_mut().push(EscapeLayer {
            id,
            show,
            opened_at,
        });
    });
    cleanup(move || {
        stack.with(|layers| {
            layers.borrow_mut().retain(|layer| layer.id != id);
        });
    });
}

/// Closes the most recently opened popup; false when none is open.
#[allow(
    clippy::must_use_candidate,
    reason = "closes the top popup; the result is optional"
)]
fn close_popup() -> bool {
    ESCAPE_LAYERS.with(|stack| {
        let top = stack
            .borrow()
            .iter()
            .copied()
            .filter(|layer| layer.show.peek())
            .max_by_key(|layer| (layer.opened_at.peek(), layer.id));
        top.is_some_and(|layer| hide(layer.show))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::runtime;

    #[test]
    fn escape_closes_most_recently_opened_popup_regardless_of_build_order() {
        runtime("popup-open-order", |_| {
            let first = signal(false);
            let second = signal(false);
            PopupLayer::new(second, PopupAnchor::Top).register_escape();
            PopupLayer::new(first, PopupAnchor::Top).register_escape();

            first.set(true);
            second.set(true);

            assert!(close_popup());
            assert!(first.peek());
            assert!(!second.peek());

            assert!(close_popup());
            assert!(!first.peek());
            assert!(!close_popup());
        });
    }
}
