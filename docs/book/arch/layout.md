# Layout engine

`sigil-layout::LayoutEngine` is the only production layout. It
mirrors a kernel subtree into an owned drive, computes boxes, and
writes `LayoutResult` back onto the same `NodeId`s.

There is no HAL `Layout` trait in the live path. Constraint types
`Dim` and `Size` live in `sigil-hal`. The engine is a concrete type.

## One box API

[`LayoutStyle`](../01-layout.md) is the only layout vocabulary.
Widgets do not own a second overflow, inset, or min-width system.
`scroll()` is Y sugar for `Overflow::Scroll`.

Style atoms (`p-4`, `.padding(16)`, `Style::padding(16)`) flatten
into that same `LayoutStyle` through `style_adapter`. Two authoring
surfaces, one box.

## Incremental compute

`LayoutEngine::compute(cx, root, available)`:

1. Drain `children_reactive` / `children_keyed` so the kernel tree
   matches current signals.
2. Sync dirty nodes into the drive (skip when `Node::version` and
   inherited text signature match).
3. Solve (flex, block, grid).
4. Attach `LayoutResult` only when the box actually moved.

A cache key of (signature, available, scale) hits skip the solve.
`LayoutDrive::new` clears the thread-local text measure memo so a
new engine does not inherit another test's shapes.

`Display::None` is a real hide. Showing again must see a reactive
style change (`StyleBinding::has_reactive`), or the forest stays
empty.

## Text measure

The engine does not shape. It asks `TextMeasure`. Production uses
`OwnedTextEngine::thread()` (same engine as caret geometry). Tests
use a stub that returns zeros or fixed widths.

nowrap + fixed-width reactive text is paint-only: it does not change
the layout signature. The box stays; the glyphs update on emit.

## What you call

App code almost never constructs `LayoutEngine`. `launch` does.
Headless tests do:

```rust,ignore
let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
layout.compute(&mut cx, root, Size::fixed(640.0, 480.0))?;
```
