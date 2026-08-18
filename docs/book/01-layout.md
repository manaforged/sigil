# The box

Every node is one box. You set [`LayoutStyle`](https://docs.rs/sigil-ui).
Widgets are sugar. They do not own a second overflow, inset, or min-width
system.

If you already know CSS, you already know this chapter.

## The closed list

```text
display     block | flex_row | flex_column | grid
size        width height min_size max_size
flex        grow shrink flex_basis align_* justify_*
overflow    overflow / overflow_x / overflow_y
            Visible | Hidden | Scroll
position    Relative | Absolute
inset       inset_top inset_right inset_bottom inset_left
space       padding padding_x padding_y margin gap
```

That is the layout vocabulary. A new layout job is a method on
`LayoutStyle`, not a new widget.

```rust,ignore
use sigil::prelude::*;

container()
    .layout(
        LayoutStyle::flex_row()
            .grow(1.0)
            .min_size(0.0, Dimension::Auto)
            .overflow(Overflow::Hidden)
            .child(/* ... */),
    )
```

Pin a handle to the trailing edge:

```rust,ignore
container().layout(
    LayoutStyle::block()
        .fixed_size(8.0, 28.0)
        .position(Position::Absolute)
        .inset_right(0.0)
        .inset_top(0.0),
)
```

## Four rules

1. **The box is the API.** Ask “what does the box do?” before you add a
   widget field. Table min-width is `min_size` on a cell. Page scroll is
   `overflow` on the page box. `scroll()` is sugar for a Y scrollport.
2. **Children keep their identity.** Update text and style on the same
   node. Do not remount an image or a path because a label changed.
   List identity is `children_keyed` — a stable key keeps the
   `NodeId`. `children_reactive` remounts by index. Mix widget types
   with `node(text("a"))`.
3. **The node that asked owns the pointer.** Capture goes to the handle
   that called `capture()`, not the deepest child under the cursor.
4. **Color follows the theme at paint time.** Stroke and fill read
   tokens (`fg`, `gold`). They do not bake an `Rgba8` at build.

## What `overflow: Scroll` is

Same job as CSS `overflow: auto`. The box clips. If content is larger,
the box pans (wheel, keys, thumb). Today `scroll()` does this on Y.
Both axes belong on the box, not on `table()` or a second scroller.

## Fill

`grow(1)` is CSS `flex: 1; min-width: 0` — Auto min-size becomes 0 so
the pane can shrink. Give a pane a literal pixel size only when you
mean a fixed pane.

## Text

`text` wraps to a definite-width box. `label` is chrome: one line.
Use `.ellipsize()` for a truncated slot. `.nowrap()` / `.wrap()`
override either.

Keep identity: `.visible_when(on)` hides with `hidden`. Do not
remount a page with `children_reactive` just to switch it.

## Authoring

`classes!("flex grow grid-cols-7")` compiles to this same box. It is
not a second layout system. The box is `LayoutStyle` (and widget
`.padding(px)`). A class token such as `p-4` fills a slot only when
the box left it at zero. If both set padding, the box wins.

A grid in a grow pane gets a definite height from its parent. `Fr`
rows share that leftover. Grid items with `Auto` height stretch to
the track (CSS `align-items: stretch`). Pin a cell height only when
you want a shorter box inside the track.

## Styling vs layout

Paint (color, radius, shadow, type) is [02 — Styling](02-styling.md).
Layout does not pick colors. Style does not set overflow.
