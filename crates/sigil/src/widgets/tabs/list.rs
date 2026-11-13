use crate::layout::Space;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::kernel::{AccessibleName, NodeBuilder, NodeId, Role, container};
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Signal, effect, memo, untrack};
use crate::style::StyleOverlay;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string_once;
use crate::widgets::Widget;

use super::super::listbox::{Move, RowSource, TAB_LIST};
use super::builder::{CloseHandler, StripChrome};
use super::drag::TabDrag;
use super::tab::{build_tab, tab_height};

type TabIds = Rc<RefCell<Vec<Option<NodeId>>>>;

#[derive(Clone)]
struct Roving {
    active: Signal<usize>,
    order: Rc<Vec<usize>>,
    enabled: Rc<Vec<bool>>,
    ids: TabIds,
}

impl RowSource for Roving {
    fn len(&self) -> usize {
        self.order.len()
    }

    fn enabled(&self, index: usize) -> bool {
        self.enabled.get(index).copied().unwrap_or(false)
    }
}

impl Roving {
    fn move_to(&self, step: Move, from: usize) {
        let Some(target) = TAB_LIST.target(self, from, step) else {
            return;
        };
        if let Some(original) = self.order.get(target) {
            self.active.set(*original);
        }
        if let Some(Some(id)) = self.ids.borrow().get(target) {
            id.focus();
        }
    }

    fn install(&self, mut node: NodeBuilder, visual: usize) -> NodeBuilder {
        for (key, step) in TAB_LIST.bindings() {
            let roving = self.clone();
            node = node.key(key, move |event| {
                roving.move_to(step, visual);
                event.stop_propagation();
            });
        }
        let ids = Rc::clone(&self.ids);
        node.added(move |_cx, id| {
            if let Some(slot) = ids.borrow_mut().get_mut(visual) {
                *slot = Some(id);
            }
        })
    }
}

#[derive(Clone)]
pub(super) struct StripSpec {
    pub(super) active: Signal<usize>,
    pub(super) disabled: Rc<Vec<usize>>,
    pub(super) order: Option<Signal<Vec<usize>>>,
    pub(super) drag: Option<TabDrag>,
    pub(super) closeable: bool,
    pub(super) close_handler: Option<CloseHandler>,
    pub(super) user_overlay: StyleOverlay,
    pub(super) chrome: StripChrome,
}

pub(super) fn build_tab_list(
    labels: Signal<Vec<String>>,
    aria_label: &str,
    spec: StripSpec,
) -> NodeBuilder {
    let active = spec.active;
    effect(move || {
        let count = labels.get().len();
        if count > 0 && active.peek() >= count {
            active.set(count - 1);
        }
    });
    let name = if aria_label.is_empty() {
        theme_string_once(StringKey::Tabs)
    } else {
        aria_label.to_owned()
    };
    container()
        .role(Role::TabList)
        .label(AccessibleName::Static(name))
        .layout(
            LayoutStyle::flex_row()
                .height(tab_height(spec.chrome))
                .shrink(0.0)
                .gap(Space::Xs)
                .align_items(AlignItems::Center),
        )
        .children_reactive(move || {
            let labels = labels.get();
            untrack(|| tab_children(&spec, &labels))
        })
}

fn tab_children(spec: &StripSpec, labels: &[String]) -> Vec<NodeBuilder> {
    let count = labels.len();
    let active = spec.active;
    let order: Vec<usize> = spec
        .order
        .map(|order| order.peek())
        .filter(|current| current.len() == count)
        .unwrap_or_else(|| (0..count).collect());
    let disabled: HashSet<usize> = spec.disabled.iter().copied().collect();
    let enabled = order
        .iter()
        .map(|original| !disabled.contains(original))
        .collect();
    let roving = Roving {
        active,
        order: Rc::new(order),
        enabled: Rc::new(enabled),
        ids: Rc::new(RefCell::new(vec![None; count])),
    };
    if let Some(drag_state) = &spec.drag {
        drag_state.bind(spec.order, count);
    }
    roving
        .order
        .iter()
        .copied()
        .enumerate()
        .map(|(visual_index, original)| {
            let label = labels.get(original).cloned().unwrap_or_default();
            let tab = build_tab(
                original,
                visual_index,
                label,
                active,
                &spec.disabled,
                spec.drag.clone(),
                spec.closeable,
                spec.close_handler.clone(),
                spec.user_overlay.clone(),
                spec.chrome,
            );
            let tab_stop = memo(move || active.get() == original);
            let tab = tab.mutate(move |node| node.a11y.tab_stop = Some(tab_stop));
            roving.install(tab, visual_index)
        })
        .collect()
}
