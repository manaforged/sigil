use crate::kernel::NodeBuilder;
use crate::reactive::{SendableSignal, memo};
use crate::style::StyleAtom;
use crate::style::install::install_reactive;
use crate::style::intern::intern;

type Live = SendableSignal<&'static [StyleAtom]>;

#[derive(Clone, Default)]
pub(crate) struct LivePaint(Vec<Live>);

impl LivePaint {
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn push<A>(&mut self, read: impl Fn() -> A + 'static)
    where
        A: IntoIterator<Item = StyleAtom>,
    {
        self.0
            .push(memo(move || intern(read().into_iter().collect::<Vec<_>>())).into_sendable());
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn atoms(&self) -> impl Iterator<Item = StyleAtom> + '_ {
        self.0
            .iter()
            .flat_map(|live| live.into_signal().get().iter().copied())
    }

    pub(crate) fn over(&self, base: &'static [StyleAtom]) -> &'static [StyleAtom] {
        if self.is_empty() {
            return base;
        }
        intern(base.iter().copied().chain(self.atoms()).collect::<Vec<_>>())
    }

    pub(crate) fn install(self, node: NodeBuilder) -> NodeBuilder {
        self.0.into_iter().fold(node, |node, read| {
            node.mutate(move |node| install_reactive(node, move || read.into_signal().get()))
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
                .all(|(a, b)| a.into_signal() == b.into_signal())
    }
}

impl Eq for LivePaint {}
