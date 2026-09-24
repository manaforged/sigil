# Content

Text, icons, images, vector paths, and charts — the leaf nodes that
paint content.

## text and label

`text(source) -> TextBuilder` where `source` is `impl Into<TextContent>`
— `&str`, `String`, `Signal<String>`, or `TextContent::compute(f)` for a
derived string. `text_live(f)` is shorthand for the compute form.
`label(source)` is `text(source).nowrap().class("text-on-surface")` —
the single-line foreground-text convenience.

```rust,ignore
let count = signal(0_i32);
label(TextContent::compute(move || format!("Count: {}", count.get())));
```

`TextBuilder` options: `.wrap()` (default) / `.nowrap()` /
`.ellipsize()` (single line with truncation), `.role(Role)` (default
`Role::Label`), `.label(name)` and `.description(text)` for
accessibility, `.focusable(bool)`, `.tab_index(i32)`,
`.keyboard_shortcut(Shortcut)`, `.hittable(bool)`, and `.mutate(f)` for
raw node edits. Font size and color come from `.class()` / `.style()`
(`"text-sm"`, `"text-*"` tone classes, …), not dedicated methods.

## icon

`icon(glyph) -> IconBuilder`. A glyph centered in a fixed box;
`Role::Image` when labeled, `Role::GenericContainer` otherwise.

- `.tone(Tone)` — glyph color.
- `.size(Size)` — Compact 16 px, Default 20 px, Wide/Grow 24 px, with
  matching text size.
- `.label(name)` — assistive name; always set it when the icon stands
  alone.

`icon_svg(svg) -> IconBuilder` draws an SVG path instead of a glyph.
The icon takes its colour from `.tone(Tone)` at paint time, so a
theme switch recolours it. Dark themes draw it light.

`icon_button(glyph, activate)` (see [Controls](controls.md)) is the
interactive counterpart.

## image

`image(source: ImageSource) -> ImageBuilder` builds an image node.
`image_rgba` and `image_bytes` return the same `ImageBuilder`.

`image_rgba(width, height, rgba: Vec<u8>) -> Option<ImageBuilder>` —
decodes raw RGBA8 into an `ImageSource`; returns `None` on bad input.
`image_bytes(encoded: &[u8]) -> Option<ImageBuilder>` decodes PNG/JPEG/
etc. through the `image` crate and requires the `image-decode` feature.

Builder options: `.alt(text)` (accessibility name),
`.fit(ObjectFit::Fill|Contain|Cover)`, `.crop(top, height)` to slice a
vertical band, plus the usual `.width()`/`.height()` layout overrides.

## path

`path(width, height) -> PathNode` — a tiny vector canvas rasterized
through tiny-skia into an image. Build with commands:

```rust,ignore
path(16, 16)
    .move_to(2.0, 8.0)
    .line_to(14.0, 8.0)
    .stroke(Rgba8 { r: 255, g: 255, b: 255, a: 255 }, 2.0)
    .fit();
```

Commands: `.move_to`, `.line_to`, `.quad_to`, `.cubic_to`, `.close`.
Paint with `.fill(color)`, `.fill_linear(start, end, stops)`,
`.fill_radial(center, radius, stops)`, and `.stroke(color, width)`.
`fill` and `stroke` take an `Rgba8` or a theme `Token`. A `Token`
resolves against the active theme at paint time.
`.layout(LayoutStyle)` overrides layout; `.fit()` / `.fit_size(px)`
fix the node to a logical size and rasterize at 2× for crispness.
Finished paths are interned in a thread-local raster cache (256
entries), so repeated identical paths share their pixmap.

## chart

`chart(values: impl IntoIterator<Item = f32>) -> ChartBuilder`. A
framed 280×180 bar chart by default.

- `.bar()` / `.line()` — mark type (`ChartMark::Bar` is the default).
- `.tone(Tone)` — recolors the first series.
- `.labels(iter)` — x-axis category labels.
- `.series(name, values, tone)` — appends another series; all series
  share the label axis.
- `.height(px)` (min 48) / `.width(px)` (min 80).

Values are a static `Vec<f32>` taken at build time — rebuild the widget
to update data. The node is `Role::GenericContainer` labeled "Chart".

## Small content helpers

These helpers from `sigil::widgets::patterns` paint fixed content:
`section_label(title)`, `empty_state(title, body)`,
`notice_band(body, tone)`, `hint_bar(keys, right)`, `avatar(initials)`,
`hairline()`, and `spacer()`. [Chrome](chrome.md#patterns) describes
each one. `divider_horizontal()` and `divider_vertical()` draw 1 px
rules in the border colour.

## States, keys, and accessibility

Content nodes have one visual state: the visual catalog captures only
the default state. They take no keys unless you add a handler, for
example `.focusable(true)` and `.key(...)` on a `text` node.

| Widget | Role | Name source |
| --- | --- | --- |
| `text`, `label` | `Label` (change it with `.role()`) | The text, or `.label(name)` |
| `icon` | `Image` with `.label()`, else `GenericContainer` | `.label(name)` |
| `image_rgba`, `image_bytes` | Image node | `.alt(text)` |
| `path` | Image node | None |
| `chart` | `Figure` | "Chart" |
| `avatar` | `Image` | The initials |
| `empty_state` | `Status` | The title and body text |
| `notice_band` | `Alert` for the error and warn tones, else `Status` | "Error", "Warning", "Success", or "Notice"; the body is the text child |

## Terminal notes

- Text keeps bold, dim, italic, underline, and strikethrough when the
  terminal supports them. Font size classes do not change the cell
  size.
- Images use the Kitty, iTerm2, or Sixel protocol when the terminal
  has one. Other terminals get sextant glyphs. See
  [Terminal host](../terminal.md).
- A `path` rasterises to an image, so it follows the image rules.
- A chart paints bars as cell fills. Keep labels short.
- A 1 px divider paints as a rule glyph, not a full cell block.

## Example

Run it with `cargo run -p sigil-ui --example widget_content`.

```rust
{{#include ../../../crates/sigil/examples/widget_content.rs}}
```
