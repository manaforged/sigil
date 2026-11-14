use crate::kernel::NodeBuilder;
use crate::reactive::{SendableSignal, Signal, memo};
use crate::style::StyleAtom;
use crate::style::builder_ext::EMPTY_ATOMS;
use crate::style::install::install_reactive;
use crate::style::intern::intern;
use crate::style::merge::SlotSet;
use crate::style::state::PerState;

#[derive(Clone, Copy, Debug)]
struct Live {
    slots: SlotSet,
    atoms: SendableSignal<&'static [StyleAtom]>,
}

#[derive(Clone, Default)]
pub(crate) struct LivePaint(Vec<Live>);

pub(crate) const NO_STATE_PAINT: PerState<LivePaint> = PerState::new(
    LivePaint::new(),
    LivePaint::new(),
    LivePaint::new(),
    LivePaint::new(),
    LivePaint::new(),
);

impl LivePaint {
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn push<A>(&mut self, read: impl Fn() -> A + 'static) -> SlotSet
    where
        A: IntoIterator<Item = StyleAtom>,
    {
        let atoms = memo(move || intern(read().into_iter().collect::<Vec<_>>()));
        let slots = SlotSet::of(atoms.peek());
        self.shadow(slots);
        self.0.push(Live {
            slots,
            atoms: atoms.into_sendable(),
        });
        slots
    }

    pub(crate) fn shadow(&mut self, slots: SlotSet) {
        if !slots.is_empty() {
            self.0.retain(|live| !live.slots.overlaps(slots));
        }
    }

    pub(crate) fn append(&mut self, other: &Self) {
        for live in &other.0 {
            self.shadow(live.slots);
            self.0.push(*live);
        }
    }

    pub(crate) fn slots(&self) -> SlotSet {
        self.0
            .iter()
            .fold(SlotSet::EMPTY, |set, live| set.union(live.slots))
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn reads(
        self,
    ) -> impl Iterator<Item = impl Fn() -> &'static [StyleAtom] + Copy + 'static> {
        self.0
            .into_iter()
            .map(|live| move || live.atoms.into_signal().get())
    }

    pub(crate) fn install(self, node: NodeBuilder) -> NodeBuilder {
        self.reads().fold(node, |node, read| {
            node.mutate(move |node| install_reactive(node, read))
        })
    }

    pub(crate) fn install_when(self, node: NodeBuilder, active: Signal<bool>) -> NodeBuilder {
        self.reads().fold(node, |node, read| {
            node.mutate(move |node| {
                install_reactive(
                    node,
                    move || if active.get() { read() } else { EMPTY_ATOMS },
                );
            })
        })
    }
}

impl std::fmt::Debug for LivePaint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LivePaint").field(&self.0.len()).finish()
    }
}

impl PartialEq for LivePaint {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(&other.0)
                .all(|(a, b)| a.slots == b.slots && a.atoms.into_signal() == b.atoms.into_signal())
    }
}

impl Eq for LivePaint {}
