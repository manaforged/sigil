A node of the tree: its children, its accessibility, and its handlers. `row`, `column`, and `container` return a `NodeBuilder`, and every widget builder implements `Into<NodeBuilder>`.

Add children with `child` or `children(iter)`. `show` switches between two subtrees, `switch` picks among keyed cases, and `for_each` renders a keyed list that keeps each row's node while its key stays. Layout, event, and accessibility setters come from [`Widget`](crate::widgets::Widget), visual setters from [`Paint`](crate::style::Paint), and interactive state setters (`.rest`, `.hover`, `.focus`, `.pressed`, `.disabled_paint`) from [`States`](crate::style::States).

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
