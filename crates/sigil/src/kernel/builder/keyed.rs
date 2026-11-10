use crate::kernel::component::IntoNode;
use crate::kernel::{NodeBuilder, reactive_children};

impl NodeBuilder {
    #[must_use]
    #[track_caller]
    pub(crate) fn keyed_children<F, I, K, N>(self, f: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: IntoIterator<Item = (K, N)>,
        K: Clone + Eq + std::hash::Hash + 'static,
        N: IntoNode + 'static,
    {
        self.keyed_children_unmounted(move |_| {
            f().into_iter().map(|(key, child)| (key, Some(child)))
        })
    }

    #[must_use]
    #[track_caller]
    pub(crate) fn keyed_children_unmounted<F, I, K, N>(mut self, f: F) -> Self
    where
        F: Fn(&dyn Fn(&K) -> bool) -> I + 'static,
        I: IntoIterator<Item = (K, Option<N>)>,
        K: Clone + Eq + std::hash::Hash + 'static,
        N: IntoNode + 'static,
    {
        let location = core::panic::Location::caller();
        self.reactive_children = Some(Box::new(move |parent| {
            reactive_children::install_keyed(parent, location, f);
        }));
        self
    }
}
