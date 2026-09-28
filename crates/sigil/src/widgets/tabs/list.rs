use std::cell::RefCell;
use std::rc::Rc;

use crate::events::prelude::{Key, NodeEventsExt};
use crate::events::request_focus;
use crate::kernel::{AccessibleName, NodeBuilder, NodeId, Role, container};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::{StyleOverlay, space};

use super::builder::{CloseHandler, StripChrome, TAB_HEIGHT};
use super::drag::DragState;
use super::tab::{build_tab, line_strip_height};

type TabIds = Rc<RefCell<Vec<Option<NodeId>>>>;

#[derive(Clone)]
struct Roving {
    active: Signal<usize>,
    order: Rc<Vec<usize>>,
    disabled: Rc<Vec<usize>>,
    ids: TabIds,
}

impl Roving {
    fn enabled(&self) -> Vec<usize> {
        (0..self.order.len())
            .filter(|visual| {
                self.order
                    .get(*visual)
                    .is_some_and(|original| !self.disabled.contains(original))
            })
            .collect()
    }

    fn move_to(&self, key: Key, from: usize) {
        let enabled = self.enabled();
        let Some(target) = roving_target(&enabled, from, key) else {
            return;
        };
        if let Some(original) = self.order.get(target) {
            self.active.set(*original);
        }
        if let Some(Some(id)) = self.ids.borrow().get(target) {
            request_focus(*id);
        }
    }

    fn install(&self, mut node: NodeBuilder, visual: usize) -> NodeBuilder {
        for key in [Key::ArrowLeft, Key::ArrowRight, Key::Home, Key::End] {
            let roving = self.clone();
            node = node.key(key, move |event| {
                roving.move_to(key, visual);
                event.stop_propagation();
            });
        }
        let ids = Rc::clone(&self.ids);
        node.on_added(move |_cx, id| {
            if let Some(slot) = ids.borrow_mut().get_mut(visual) {
                *slot = Some(id);
            }
        })
    }
}

fn roving_target(enabled: &[usize], from: usize, key: Key) -> Option<usize> {
    let len = enabled.len();
    let at = enabled.iter().position(|visual| *visual == from);
    let index = match (key, at) {
        (Key::Home, _) => 0,
        (Key::End, _) => len.checked_sub(1)?,
        (Key::ArrowRight, Some(at)) => (at + 1) % len,
        (Key::ArrowLeft, Some(at)) => (at + len - 1) % len,
        _ => return None,
    };
    enabled.get(index).copied()
}

fn accessible_label(label: &str) -> AccessibleName {
    if label.is_empty() {
        AccessibleName::Static(String::from("Tabs"))
    } else {
        AccessibleName::Static(label.to_owned())
    }
}

fn tab_list_layout(chrome: StripChrome) -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(if matches!(chrome, StripChrome::Line) {
            line_strip_height()
        } else {
            TAB_HEIGHT
        })
        .gap(space(1))
        .align_items(AlignItems::Center)
}

fn visual_order(order: Option<Signal<Vec<usize>>>, count: usize) -> Vec<usize> {
    order
        .map(|sig| sig.peek())
        .filter(|current| current.len() == count)
        .unwrap_or_else(|| (0..count).collect())
}

#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "the argument list mirrors the upstream API contract"
)]
pub(super) fn build_tab_list(
    active: Signal<usize>,
    labels: Vec<String>,
    disabled: &[usize],
    aria_label: &str,
    order: Option<Signal<Vec<usize>>>,
    drag: Option<DragState>,
    closeable: bool,
    close_handler: Option<CloseHandler>,
    user_overlay: StyleOverlay,
    chrome: StripChrome,
) -> NodeBuilder {
    let count = labels.len();
    let disabled = Rc::new(disabled.to_vec());
    let roving = Roving {
        active,
        order: Rc::new(visual_order(order, count)),
        disabled: Rc::clone(&disabled),
        ids: Rc::new(RefCell::new(vec![None; count])),
    };
    let mut root = container()
        .role(Role::TabList)
        .label(accessible_label(aria_label))
        .layout(tab_list_layout(chrome));
    for (visual_index, original) in roving.order.iter().copied().enumerate() {
        let label = labels.get(original).cloned().unwrap_or_default();
        let tab = build_tab(
            original,
            visual_index,
            label,
            active,
            &disabled,
            drag,
            closeable,
            close_handler.as_ref().map(Rc::clone),
            user_overlay.clone(),
            chrome,
        );
        let tab_stop = derived(move || active.get() == original);
        let tab = tab.mutate(move |node| node.a11y.tab_stop = Some(tab_stop));
        root = root.child(roving.install(tab, visual_index));
    }
    if let Some(drag_state) = drag {
        drag_state.bind(order, count);
    }
    root
}
