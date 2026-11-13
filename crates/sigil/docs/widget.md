The setters every builder shares: layout, style, focus, animation, and drag and drop.

Spacing takes a step of the scale (`Space::Md`), text takes a role (`TextRole::Title`), and colors take a theme token (`Token::Accent`). A bare pixel value does not compile where a step is expected.

```rust
use sigil::prelude::*;

fn toolbar() -> impl IntoNode {
    row()
        .gap(Space::Sm)
        .padding(Space::Md)
        .background(Token::Surface)
        .child(button("Save").primary())
        .child(button("Cancel"))
}
```

Implement [`Framed`](crate::widgets::Framed) and [`IntoNode`](crate::node::IntoNode) on a custom builder to inherit `Widget`, [`Paint`](crate::style::Paint), and [`States`](crate::style::States).

See also: [`NodeBuilder`](crate::node::NodeBuilder), [Recipes](https://github.com/manaforged/sigil/blob/main/docs/guide/recipes.md).
