# Node tree

`sigil-kernel` owns the UI tree. `Cx` is the tree context. `NodeId` is
stable until you remove the node.

`Cx` is the only context authors see. `sigil_reactive::EntityCx` is a
hidden Entity update token. Do not import it.

## Building

```rust,ignore
fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(container().child(text("Hi")))
}
```

Each `add` creates a component owner (see [Owner tree](owners.md)).
`remove` disposes that owner and the subtree.

Builders (`container`, `text`, `button`, …) implement `IntoNode`.
Widgets return a builder. `cx.add` consumes it.

## Identity

`children_reactive` remounts children **by index**. A length change
rebuilds the list. Same length, same slots: the factory runs, but
keyed keep (below) is a different API.

`children_keyed` remounts by **key**. A matching key keeps the
`NodeId`. The new builder for a kept key is dropped (a dummy
`container()` is enough). First key wins on duplicates.

```rust,ignore
parent.children_keyed(move || {
    items.get().into_iter().map(|row| (row.id, row_view(row))).collect()
})
```

`virtual_list(items, render)` takes a `Vec` or a `Signal`. Live
lists use keyed rows. Default keys are list indices. If
`T: Identifiable`, call `.identified()` so a reorder keeps
`NodeId`. `.key_by` is the escape when the key is not
`Identifiable::id`. Same id as `TableState::identified`.

Static `.child()` nodes stay as a prefix in front of a reactive
list.

## Text

`text` wraps. `label` is chrome (one line). Content is
`TextContent`:

- `Static` — a string baked at build
- `Reactive` — a `Signal<String>` (or a `derived` converted at
  `TextBuilder::build`)

A factory that runs before the node owner exists must not call
`derived()` itself. Use `text_live` in that factory; build converts
the closure to a node-owned `derived`. Live nodes store `Reactive`
only.

## Pointer

The node that called `capture()` owns the pointer, not the deepest
child under the cursor. That is a locked layout/event rule (see
[The box](../01-layout.md)).
