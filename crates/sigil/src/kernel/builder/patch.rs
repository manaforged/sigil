use ahash::AHashSet;
use smallvec::SmallVec;

use crate::events::EventHandlers;
use crate::kernel::component::{ErasedComponent, Patch};
use crate::kernel::{A11yProps, Cx, NodeBuilder, NodeId, NodeType, TopLayer, reactive_children};
use crate::profile::Counter;
use crate::reactive::{Owner, is_live};

type Ids = SmallVec<[NodeId; 4]>;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Fate {
    Patched(NodeId),
    Mounted(NodeId),
    Kept(NodeId),
}

impl Fate {
    pub(crate) const fn id(self) -> NodeId {
        match self {
            Self::Patched(id) | Self::Mounted(id) | Self::Kept(id) => id,
        }
    }

    pub(crate) const fn is_fresh(self) -> bool {
        !matches!(self, Self::Kept(_))
    }
}

impl Cx {
    pub(crate) fn kind_matches(&self, id: NodeId, kind: &NodeType) -> bool {
        self.nodes
            .get(id.key())
            .is_some_and(|node| std::mem::discriminant(&node.kind) == std::mem::discriminant(kind))
    }

    pub(crate) fn refill(&mut self, id: NodeId, fill: impl FnOnce(&mut Self)) {
        if let Some(old) = self.owner_for_node.remove(&id)
            && is_live(old.id())
        {
            old.dispose();
        }
        let parent_owner = Owner::current().unwrap_or_else(|| self.root_owner());
        let owner = Owner::new("sigil-kernel::component", Some(parent_owner));
        let before = self.indexed(id);
        owner.with(|| fill(self));
        self.owner_for_node.insert(id, owner);
        if before || self.indexed(id) {
            self.mutation_generation = self.mutation_generation.wrapping_add(1);
        }
    }

    fn indexed(&self, id: NodeId) -> bool {
        self.nodes.get(id.key()).is_some_and(|node| {
            node.a11y.focusable
                || node.a11y.traps_tab
                || node.a11y.tab_index.is_some()
                || node.a11y.keyboard_shortcut.is_some()
                || node.get_extension::<TopLayer>().is_some()
                || node.get_extension::<EventHandlers>().is_some()
        })
    }

    pub(crate) fn reset_node(&mut self, id: NodeId, kind: NodeType, a11y: A11yProps) {
        let owner = Owner::current().unwrap_or_else(|| self.root_owner());
        if let Some(node) = self.get_mut(id) {
            node.kind = kind;
            node.a11y = a11y;
            let placed = node.remove_extension::<crate::layout::LayoutResult>();
            node.extensions = None;
            if let Some(placed) = placed {
                let _previous = node.attach_untracked(placed);
            }
            node.owner = owner;
            node.bump_version();
        }
    }

    pub(crate) fn reconcile_child(
        &mut self,
        parent: NodeId,
        existing: Option<NodeId>,
        builder: Option<Box<dyn ErasedComponent>>,
    ) -> Option<Fate> {
        let live = existing.filter(|id| self.nodes.contains_key(id.key()));
        let Some(builder) = builder else {
            return live.map(Fate::Kept);
        };
        let Some(id) = live else {
            return Some(self.mount_child(parent, builder));
        };
        match builder.patch_box(self, id) {
            Patch::Patched => {
                crate::profile::count_add(Counter::KernelPatched, 1);
                Some(Fate::Patched(id))
            }
            Patch::Mismatch(builder) | Patch::Unsupported(builder) => {
                self.remove_counted(id);
                Some(self.mount_child(parent, builder))
            }
        }
    }

    pub(crate) fn remove_counted(&mut self, id: NodeId) {
        if self.remove(id) {
            crate::profile::count_add(Counter::KernelRemoved, 1);
        }
    }

    pub(crate) fn others(&self, parent: NodeId, keep: &AHashSet<NodeId>) -> Ids {
        self.nodes
            .get(parent.key())
            .map(|node| {
                node.children
                    .iter()
                    .copied()
                    .filter(|id| !keep.contains(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn mount_child(&mut self, parent: NodeId, builder: Box<dyn ErasedComponent>) -> Fate {
        crate::profile::count_add(Counter::KernelMounted, 1);
        Fate::Mounted(self.add_boxed_child(parent, builder))
    }
}

impl NodeBuilder {
    pub(super) fn patch_onto(mut self, cx: &mut Cx, id: NodeId) -> Patch<Self> {
        if self.wraps.is_some() {
            return Patch::Unsupported(self);
        }
        if !cx.kind_matches(id, &self.kind) {
            return Patch::Mismatch(self);
        }
        cx.refill(id, |cx| {
            let style = std::mem::take(&mut self.style);
            let mut this = crate::style::attach_style(self, style);
            let kind = std::mem::replace(&mut this.kind, NodeType::Container);
            cx.reset_node(id, kind, std::mem::take(&mut this.a11y));
            this.refill_onto(cx, id);
        });
        Patch::Patched
    }

    fn refill_onto(self, cx: &mut Cx, id: NodeId) {
        if let Some(node) = cx.get_mut(id) {
            for apply in self.pre_extensions {
                apply(node);
            }
        }
        let old: Ids = cx
            .get(id)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        let mut old = old.into_iter();
        let mut kept = AHashSet::new();
        let mut order = Ids::new();
        for child in self.children {
            if let Some(fate) = cx.reconcile_child(id, old.next(), Some(child)) {
                kept.insert(fate.id());
                order.push(fate.id());
            }
        }
        let mut rest: Ids = old.collect();
        if let Some(install) = self.reactive_children {
            reactive_children::seed(id, rest);
            install(id);
            rest = reactive_children::take_seed(id);
        }
        for orphan in rest {
            cx.remove_counted(orphan);
        }
        let tail = cx.others(id, &kept);
        cx.replace_children(id, order.iter().chain(&tail).copied().collect());
        for hook in self.on_added {
            hook(cx, id);
        }
    }
}
