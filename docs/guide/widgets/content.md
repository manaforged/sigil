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

`icon_button(glyph, activate)` (see [Controls](controls.md)) is the
interactive counterpart.

## image

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
Paint with `.fill(Rgba8)`, `.fill_linear(start, end, stops)`,
`.fill_radial(center, radius, stops)`, and `.stroke(color, width)`.
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
