use std::hash::Hash;

use crate::kernel::{IntoNode, NodeBuilder, fragment, node};
use crate::reactive::{Reactive, Signal};

#[must_use]
pub fn show<A, B>(
    cond: impl Reactive<bool>,
    then: impl Fn() -> A + 'static,
    otherwise: impl Fn() -> B + 'static,
) -> NodeBuilder
where
    A: IntoNode + 'static,
    B: IntoNode + 'static,
{
    fragment().children_keyed(move || {
        let branch = cond.read();
        let child = if branch {
            node(then())
        } else {
            node(otherwise())
        };
        [(branch, child)]
    })
}

#[must_use]
pub fn for_each<T, K, N>(
    items: Signal<Vec<T>>,
    key: impl Fn(&T) -> K + 'static,
    render: impl Fn(&T) -> N + 'static,
) -> NodeBuilder
where
    T: 'static,
    K: Clone + Eq + Hash + 'static,
    N: IntoNode + 'static,
{
    fragment().children_keyed(move || {
        items.with(|all| {
            all.iter()
                .map(|item| (key(item), render(item)))
                .collect::<Vec<_>>()
        })
    })
}
