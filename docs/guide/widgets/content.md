# Content

Text, icons, images, vector paths, and charts: the leaf nodes that paint content.

## text

`text(source) -> TextBuilder` is the one text node. `source` is `impl Into<TextContent>`: a `&str`, `&String`, `String`, or `Cow<'_, str>` (static), a `Signal<T: Display>` (live), or a closure `Fn() -> S` where `S: Into<String>` (such as `String` or `&str`) that re-runs when the signals it reads change. `.nowrap()` keeps it on one line.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let count = signal(0_i32);
text(move || format!("Count: {}", count.get())).nowrap();
# };
```

`TextBuilder` options: `.wrap()` (default) / `.nowrap()` / `.ellipsize()` (single line with truncation), `.role(Role)` (default `Role::Label`), `.accessible(name)` and `.description(text)` for accessibility, `.focusable(bool)`, `.tab_index(i32)`, `.accessible_shortcut(Shortcut)`, `.hittable(bool)`, and `.mutate(f)` for raw node edits. Font and color come from the `Paint` methods: `.font_size(14.0)`, `.font_weight(600)`, `.font_family(FontFamily::Mono)` (`sigil::style::FontFamily`), `.text_color(Token::OnSurfaceMuted)`.

## icon

`icon(glyph) -> IconBuilder`. A glyph centered in a fixed box; `Role::Image` when labeled, `Role::GenericContainer` otherwise.

- `.tone(Tone)`: glyph color.
- `.size(ControlSize)`: Compact 16 px, Default 20 px, Wide 24 px, with matching text size.
- `.accessible(name)`: assistive name; always set it when the icon stands alone.

`icon(Glyph::svg(source))` draws an SVG image instead of a glyph. The icon takes its color from `.tone(Tone)` at paint time, so a theme switch recolours it. Dark themes draw it light.

`button(Icon::X)` (see [Controls](controls.md)) is the interactive counterpart.

`sigil::icons::Icon`, also in `sigil::prelude`, holds the bundled icons. Each variant is the Lucide name in camel case: `Icon::X` for `x`, `Icon::ChartColumn` for `chart-column`. `Icon::ALL` lists them and `Icon::name` returns the Lucide name.

## image

`image(source: ImageSource) -> ImageBuilder` builds an image node.

`ImageSource::from_rgba8(width, height, rgba: Vec<u8>) -> Option<ImageSource>` takes raw RGBA8 and returns `None` on bad input. `ImageSource::decode(encoded: &[u8]) -> Option<ImageSource>` decodes PNG, JPEG, and other formats through the `image` crate and requires the `image-decode` feature.

Builder options: `.accessible(text)` (accessibility name), `.accessibility_hidden(true)` for a decorative image, `.fit(ObjectFit::Fill|Contain|Cover)`, `.crop(top, height)` to slice a vertical band, plus the usual `.width()`/`.height()` layout overrides.

## load_image

`load_image(location) -> ImageLoadBuilder` (feature `image-decode`). Loads a file path, a `file://` URL, or a base64 `data:` URI off the UI thread. It also fetches http(s) URLs in a browser, and in native builds with the `image-http` feature, with a 30 s timeout, a 64 MiB body limit, and an `ErrorKind::Http` error for an error status. Size it with `.width(..)` and `.height(..)`: on a sized axis the loaded image fills the box with `.fit(ObjectFit)`. `.alt(text)`, `.placeholder(Fn() -> impl IntoNode)` for the loading view, and `.error(Fn(&Error) -> impl IntoNode)` for the failure view. Loads are cached by location and, for a file, its modified time and size. `ImageSource::load(location)` returns the load as a `Signal<Status<ImageSource, Error>>`.

## path

`path(width, height) -> PathBuilder`: a tiny vector canvas rasterized through tiny-skia into an image. Build with commands:

```rust,no_run
# use sigil::prelude::*;
# let _ = {
path(16, 16)
    .move_to(2.0, 8.0)
    .line_to(14.0, 8.0)
    .stroke(Color { r: 255, g: 255, b: 255, a: 255 }, 2.0)
    .fit();
# };
```

Commands: `.move_to`, `.line_to`, `.quad_to`, `.cubic_to`, `.close`. Paint with `.fill_color(color)`, `.fill_linear(start, end, stops)`, `.fill_radial(center, radius, stops)`, and `.stroke(color, width)`. `fill_color` and `stroke` take a `Color` or a theme `Token`. A `Token` resolves against the active theme at paint time. The layout setters, such as `.width(..)`, override the layout; `.fit()` / `.fit_size(px)` fix the node to a logical size. The `width` and `height` given to `path` are the viewbox: the renderer maps it onto the laid-out box and rasterizes at the physical pixel size, so paths stay sharp at any device scale. Stroke widths scale with the viewbox; call `.non_scaling_stroke()` to keep them in logical px with round joins and caps, as data lines in a stretched plot need. Rasters are interned in a thread-local cache (256 entries), keyed by path, paint, and pixel size.

## chart

`chart(values: impl IntoIterator<Item = f32>) -> ChartBuilder`. A framed 280×180 bar chart by default.

- `.bar()` / `.line()`: mark type (bars are the default).
- `.tone(Tone)`: recolors the first series.
- `.labels(iter)`: x-axis category labels.
- `.series(name, values, tone)`: appends another series; all series share the label axis.
- `.height(px)` (min 48) / `.width(px)` (min 80).

Values are `impl Reactive<Vec<f32>>`: a `Vec`, an array, a signal, or a closure. A signal or closure source draws the chart again when it changes. The node is `Role::Figure` labeled "Chart".

## stat

`stat(label, value) -> StatBuilder`. A KPI tile in card chrome showing a label and a large value, with optional:

- `.delta(change, tone)`: change line below the value in `Tone`.
- `.caption(text)`: muted caption below the value and delta.
- `.trend(values)`: `impl Reactive<Vec<f32>>` sparkline at the bottom of the tile.
- `.range(low, high)`: fixes the bottom and top of the trend sparkline.
- `.tone(Tone)`: sets the color of the value text and trend sparkline.

## sparkline

`sparkline(values: impl Reactive<Vec<f32>>) -> SparklineBuilder`. A compact inline trend graph.

- `.mark(SparkMark)`: `SparkMark::Block` (default, one value per cell) or `SparkMark::Braille` (two values per cell).
- `.fill(SparkFill)`: `SparkFill::Solid` (default) or `SparkFill::Shade` (shaded cells below the top edge).
- `.range(low, high)`: sets the value bounds; defaults to the data bounds and 0.
- `.tone(Tone)` / `.tones(iter)`: base tone (default `Tone::Primary`) or per-value `Option<Tone>`.
- `.rows(usize)`: height in rows (default 1).

## canvas

`canvas(paint: impl Fn(&mut SceneBuilder, Rect, &Theme) + 'static) -> CanvasBuilder`. A custom drawing surface that records primitives directly onto a `sigil::scene::SceneBuilder` inside the laid-out `Rect`. The `&Theme` is the active theme, so `theme.color(token)` resolves a token to a color. Chain `.redraw_on(Signal<u64>)` to repaint when a version signal changes. `CanvasBuilder` takes the `Paint`, `States`, and `Widget` methods.

- `rect` is the laid-out box of the canvas in logical px, in the same coordinates as the rest of the scene. It is not local to the canvas: its origin is the top-left corner of the canvas, so offset each primitive by `rect.origin`. Drawing outside `rect` is clipped.
- To redraw after the data changes, read the data with `peek` inside the closure and bump the `.redraw_on` signal, such as `version.update(|n| *n += 1)`. The example `story-11-canvas-chart` does this.
- To draw a vector path, build it with `path(width, height)` and the commands in [path](#path), then call `.draw(scene, rect, theme)`. The path's viewbox maps onto `rect`. `.build_path(rect, theme)` returns the `Option<DrawPath>` instead, for `scene.draw_path(draw)`; it is `None` when the path has no commands or no paint.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
canvas(|scene, rect, theme| {
    scene.fill_rect(rect, theme.color(Token::Accent));
})
.width(120.0)
.height(48.0);
# };
```

## Small content helpers

These helpers paint fixed content. They are in `sigil::prelude` and `sigil::widgets`: `section_label(title)`, `empty_state(title)`, `notice_band(body, tone)`, `hint_bar(keys, right)`, `avatar(initials)`, and `spacer()`. [Chrome](chrome.md#small-composites) describes each one. `divider(Orientation::Horizontal)` and `divider(Orientation::Vertical)` draw 1 px rules in the border color; `divider(Orientation::Vertical).height(px)` draws a short tick, such as between toolbar items.

## States, keys, and accessibility

Content nodes have one visual state: the visual catalog captures only the default state. They take no keys unless you add a handler, for example `.focusable(true)` and `.key(...)` on a `text` node.

| Widget | Role | Name source |
| --- | --- | --- |
| `text` | `Label` (change it with `.role()`) | The text, or `.accessible(name)` |
| `icon` | `Image` with `.accessible()`, else `GenericContainer` | `.accessible(name)` |
| `image` | Image node | `.accessible(text)` |
| `path` | Image node | None |
| `chart` | `Figure` | "Chart" |
| `avatar` | `Image` | The initials |
| `empty_state` | `Status` | The title |
| `notice_band` | `Alert` for the error and warn tones, else `Status` | "Error", "Warning", "Success", or "Notice"; the body is the text child |

## Terminal notes

- Text keeps bold, dim, italic, underline, and strikethrough when the terminal supports them. `font_size` does not change the cell size.
- Images use the Kitty, iTerm2, or Sixel protocol when the terminal has one. Other terminals get sextant glyphs. See [Terminal host](../terminal.md).
- A `path` rasterizes to an image, so it follows the image rules.
- A chart paints bars as cell fills. Keep labels short.
- A 1 px divider paints as a rule glyph, not a full cell block.

## Example

Run it with `cargo run -p sigil-ui --example widget_content`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_content.rs}}
```
