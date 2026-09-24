# Units

A layout length in Sigil is one of two kinds:

- A **text-relative** length follows the text grid. Use it for anything that holds text: row heights, control heights, text indents, and gaps between lines.
- A **pixel** length is a fixed size. Use it only for detail that has no text meaning, such as an icon, a hairline, or an image.

## Text-relative units

| Function | Meaning on a window | Meaning in a terminal |
| --- | --- | --- |
| `rows(n)` | `n` line heights of the default font | `n` cells |
| `cols(n)` | `n` average advances of the default font | `n` cells |
| `em(n)` | `n` times the default font size | `n` cells |

A terminal cell is 1 row by 1 col. The layout pass resolves each unit when the node enters the layout tree, so the same code gives a one-row control in a terminal and a text-height control on a window.

The functions return a `TextLength`. Every `LayoutStyle` builder that takes a `Dimension` or a `LengthPercentage` accepts it:

```rust
use sigil::prelude::*;

let row = LayoutStyle::flex_row()
    .height(rows(1.0))
    .padding_x(cols(1.0))
    .gap(cols(2.0));
```

## Pixel units

A plain `f32` is a pixel length. In a terminal, the layout pass rounds a pixel length to whole cells, and it keeps a length that is smaller than one cell as detail. Do not use pixels to guess a cell count. Use `rows` and `cols`.

## The rule

1. If the length is measured in lines or characters, use `rows`, `cols`, or `em`.
2. If the length is a fixed picture size, use pixels.
3. Do not branch on the host to pick a size.
