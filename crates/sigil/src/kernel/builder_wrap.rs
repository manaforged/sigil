use crate::kernel::component::ErasedComponent;
use crate::kernel::{A11yProps, AccessibleName, Cx, NodeBuilder, NodeId};

impl NodeBuilder {
    pub(super) fn build_onto(mut self, widget: Box<dyn ErasedComponent>, cx: &mut Cx) -> NodeId {
        let id = widget.build_box(cx);
        if let Some(node) = cx.get_mut(id) {
            overlay_a11y(&mut node.a11y, std::mem::take(&mut self.a11y));
        }
        self.finish(cx, id)
    }

    pub(super) fn finish(self, cx: &mut Cx, id: NodeId) -> NodeId {
        if let Some(node) = cx.get_mut(id) {
            for apply in self.pre_extensions {
                apply(node);
            }
        }
        for child in self.children {
            let _ = cx.add_boxed_child(id, child);
        }
        if let Some(install) = self.reactive_children {
            install(id);
        }
        for hook in self.on_added {
            hook(cx, id);
        }
        id
    }
}

fn overlay_a11y(base: &mut A11yProps, mut edits: A11yProps) {
    let defaults = A11yProps::default();
    if edits.role != defaults.role {
        base.role = edits.role;
    }
    if !matches!(edits.name, AccessibleName::None) {
        base.name = std::mem::replace(&mut edits.name, AccessibleName::None);
    }
    base.focusable |= edits.focusable;
    base.traps_tab |= edits.traps_tab;
    base.hittable &= edits.hittable;
    base.actions.extend(std::mem::take(&mut edits.actions));
    base.owns.extend(std::mem::take(&mut edits.owns));
    overlay_options(base, edits);
}

fn keep<T>(base: &mut Option<T>, edit: Option<T>) {
    if edit.is_some() {
        *base = edit;
    }
}

fn overlay_options(base: &mut A11yProps, edits: A11yProps) {
    keep(&mut base.description, edits.description);
    keep(&mut base.disabled, edits.disabled);
    keep(&mut base.tab_index, edits.tab_index);
    keep(&mut base.tab_stop, edits.tab_stop);
    keep(&mut base.keyboard_shortcut, edits.keyboard_shortcut);
    keep(&mut base.checked, edits.checked);
    keep(&mut base.selected, edits.selected);
    keep(&mut base.value_now, edits.value_now);
    keep(&mut base.value_min, edits.value_min);
    keep(&mut base.value_max, edits.value_max);
    keep(&mut base.step, edits.step);
    keep(&mut base.text_value, edits.text_value);
    keep(&mut base.text_cursor, edits.text_cursor);
    keep(&mut base.placeholder, edits.placeholder);
    keep(&mut base.labelled_by, edits.labelled_by);
    keep(&mut base.described_by, edits.described_by);
    keep(&mut base.invalid, edits.invalid);
    keep(&mut base.required, edits.required);
    keep(&mut base.live, edits.live);
    keep(&mut base.expanded, edits.expanded);
    keep(&mut base.level, edits.level);
    keep(&mut base.sort_direction, edits.sort_direction);
    keep(&mut base.row_count, edits.row_count);
    keep(&mut base.column_count, edits.column_count);
    keep(&mut base.row_index, edits.row_index);
    keep(&mut base.column_index, edits.column_index);
    keep(&mut base.position_in_set, edits.position_in_set);
    keep(&mut base.size_of_set, edits.size_of_set);
    keep(&mut base.focus_scope, edits.focus_scope);
}
