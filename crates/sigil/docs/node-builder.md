A node of the tree: its children, its accessibility, and its handlers. `row`, `column`, `container`, and most widgets return one.

Add children with `child`. `show` switches between two subtrees, and `for_each` renders a keyed list that keeps each row's node while its key stays. Layout and style setters come from [`Widget`](crate::widgets::Widget).

```rust
use sigil::prelude::*;

fn list() -> impl IntoNode {
    let items = signal(vec!["one", "two"]);
    let open = signal(true);
    column()
        .label("Items")
        .child(for_each(items, |item: &&str| *item, |item: &&str| text(*item)))
        .child(show(open, || text("Open"), || text("Closed")))
}
```

See also: [`Signal`](crate::reactive::Signal), [Widgets](https://github.com/manaforged/sigil/blob/main/docs/guide/widgets.md).
