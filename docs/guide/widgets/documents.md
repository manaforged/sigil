# Documents

Parsers and viewers for structured text: markdown, HTML, source code, diffs, diagrams, formulas and charts. `markdown`, `html`, `mermaid`, `graphviz` and `math` bound their input. A source over 1 MiB renders as a "Source is larger than the 1 MiB limit." paragraph instead of parsing; `diff` parses unbounded.

## markdown

`markdown(Signal<String>) -> NodeBuilder`. Parses the signal into blocks inside a `derived`, so the document re-renders only when the parsed tree changes. The root is `Role::Document`, a vertical column.

Parsing is CommonMark plus GitHub Flavored Markdown through `pulldown-cmark`. Blocks: headings h1–h6, paragraphs, nested ordered and bullet lists, task lists, block quotes, GFM alerts (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`) as tinted callouts with a title, rules, fenced and indented code, and tables with column alignment from the delimiter row. A table sorts by a header click, and numeric columns sort numerically. In a window it also shows Copy Markdown and Copy CSV actions. `<details><summary>…</summary>…</details>` renders as a collapsed section. Other raw HTML renders as text. Footnotes render as marks with a footnotes section at the end. Display math renders through `math()`: a `$$…$$` block, or a paragraph that is only one `$…$` or `\[…\]` formula. Inline `$…$` and `\(…\)` render as Unicode math. A `$` next to a digit or a word (`$5`, `$HOME`) stays text. Sigil-aware fences: ` ```mermaid ` renders through `mermaid()`; ` ```dot `, ` ```graphviz ` and ` ```gv ` through `graphviz()`; ` ```vega-lite ` through Vega 6.4.0 and Vega-Lite 6.5.0, which run in an embedded JavaScript runtime off the UI thread and draw an SVG in the theme's colors and font; ` ```chart ` parses an optional title plus `label: value` rows into a `chart()`; and ` ```xychart ` reads the mermaid xychart form. A chart fence that fails to parse falls back to the mermaid path. A Vega-Lite chart shows a placeholder until it is drawn. It loads no data URLs. A spec that runs longer than 2 seconds, needs more than 256 MB, or logs a Vega error falls back to the Vega-Lite subset of `chart()`, and a spec outside that subset shows as JSON code. In a terminal, the subset of `chart()` draws the spec. Inline spans: bold, italic, strikethrough, code, links, `<url>` autolinks, and bare `https://` and `www.` URLs. While the source streams in, finished blocks keep their nodes, and the last block closes a dangling `**`, `*`, `~~`, backtick or `[text](` so raw markers do not flash.

Images resolve `src` only as a `data:` base64 URI (SVG excluded). The widget never reads the filesystem or fetches a URL, so untrusted text cannot make it load a file. Images display at most 480×384 px. Decoding needs the `image-decode` feature. Without it, or on failure, the image renders as `[alt]` text.

`markdown_inline(&str) -> TextBuilder` renders one paragraph of inline spans as a single wrapped text node. Code spans keep their background, and the caller's `Paint` methods style the rest of the paragraph.

`markdown_edit(Signal<String>)` is the editor split: `text_field(source).multiline()` beside `markdown(source)` inside a horizontal `splitter`, each pane starting at 360 px. The preview follows the signal live. `.disabled(impl Reactive<bool>)` disables the editor pane.

## html

`html(Signal<String>) -> NodeBuilder`. Parses a bounded HTML subset into the same `Block` tree as markdown (tables get the same sort and copy actions, left-aligned) and renders through the same `document` view: headings, paragraphs, lists, code, tables, images. `<script>` and `<style>` blocks are stripped entirely; unknown tags degrade to their text content. It is a viewer for trusted markup, not a browser engine.

## code_edit

`code_edit(lines, caret)` where `lines` is `Signal<Vec<String>>` and `caret` is `Signal<(usize, usize)>` as `(row, column)`. A column is a grapheme cluster index, the same unit `text_field` uses, so the caret and selection never split a grapheme. `Highlight` and `Diagnostic` columns use the same unit. A line-oriented code editor that renders only the rows inside the scroll viewport. `first_row` derives the visible window from `scroll_y`, and an effect follows the caret so typing scrolls.

- `.highlights(Signal<Vec<Vec<Highlight>>>)`: per-line syntax spans; `Highlight::new(start, end, token)` maps a column range to a token class.
- `.selection(Signal<Option<(usize, usize)>>)`: selection anchor; the range from anchor to caret paints selected.
- `.gutter(Signal<Vec<Option<Change>>>)`: per-line `Added` / `Modified` / `Deleted` bars in the line-number gutter (sized to the line count: `18 + digits × 7` px).
- `.diagnostics(Signal<Vec<Diagnostic>>)`: `Diagnostic::new(line, start, end, severity)` tints a line range by `Severity` (Hint/Info → info, Warning → warn, Error → error).
- `.key_handler(Fn(&mut KeyEvent))`: a key handler that runs before the built-in editing keys. Call `event.stop_propagation()` to replace a built-in binding. Installing one traps Tab for the handler instead of moving focus.
- `.row_height(px)`: defaults to 18 px, or one cell on a terminal.
- `.disabled(impl Reactive<bool>)`: ignores keys, removes the editor from the focus order, and paints it at the disabled opacity.

The widget edits the `lines`, `caret`, and `.selection()` signals through the `text_field` controller: typing, Enter, Backspace, Delete, arrow keys, Home, End, Page Up, Page Down, Shift selection, word motion, clipboard, undo and redo, and IME commit.

## mermaid

`mermaid(source: impl Into<String>) -> NodeBuilder`. In a window it renders through the `merman` crate, which covers every mermaid 12 diagram kind, including agentflow, architecture, venn, wardley and railroad, except layouts that need ELK. The diagram takes its colors from the active theme: series and slices use the accent, success, warn, error and info scales, and text uses the bundled Fira Sans. A diagram wider than its column scales down to fit, to 60% at the smallest, and scrolls sideways past that. Hover or focus shows a toolbar: Show code / Show diagram, Copy source, and Expand diagram. Expand opens the diagram in a dialog sized to the window, with Zoom in, Zoom out and Fit; Ctrl or Cmd with the wheel zooms, the wheel or a drag pans, and Escape closes. While a `mermaid(signal)` source changes and does not parse, the last diagram that rendered stays on screen, dimmed and marked "Updating…". In a terminal, or when `merman` rejects the source and nothing has rendered yet, it draws a monospace text canvas. That path supports `graph`/`flowchart` (and `stateDiagram` / `stateDiagram-v2`), `sequenceDiagram`, and `pie`, and frames any other source as raw text.

## graphviz

`graphviz(source: impl Reactive<String>) -> NodeBuilder`. In a window it lays Graphviz DOT out with the `layout-rs` crate and draws it in theme colors: nodes in the accent scale, edges in the muted text color. Explicit `color` and `fillcolor` attributes override the theme. `layout-rs` supports box, circle, doublecircle, record and Mrecord shapes, and does not draw cluster boxes or apply `fontcolor`. In a terminal, or when the DOT does not parse, it shows the framed source.

## math

`math(source: impl Reactive<String>) -> NodeBuilder`. In a window it typesets a LaTeX formula in display style with the RaTeX crates (`ratex-parser`, `ratex-layout`, `ratex-svg`) and the KaTeX fonts they embed, in the theme text color, at about 1.25 times body size. It reads KaTeX syntax, including environments such as `align*`, `gather*` and `dcases`, and commands such as `\operatorname*`, `\middle`, `\pmod` and `\ce`. `\label` and `\iddots` do not parse. In a terminal, or when the formula does not parse, it shows a Unicode approximation. The accessible description is the LaTeX source.

Input is bounded to 256 lines of 512 chars and output to 512 lines of 1024 chars.

## diff

`diff(patch: impl Reactive<String>) -> DiffBuilder`. Parses a unified diff and renders it as split (side-by-side) or unified rows with a header showing the path and `+added`/`−removed` counts.

- `.path(text)`: header label; falls back to the patch's own paths.
- `.status(text)`: extra header label (e.g. "modified").
- `.view(DiffView::Split | DiffView::Unified)`: default Split.
- `.max_rows(n)`: truncates rendered rows.

## States, keys, and accessibility

The visual catalog captures `code_edit` and `markdown_edit` in default, hover, focused, and disabled. `markdown`, `html`, `mermaid`, and `diff` are static views.

| Widget | Keys | Role | Name source |
| --- | --- | --- | --- |
| `markdown`, `html` | none | `Document`, with `Heading`, `Paragraph`, `List`, `ListItem`, `Code`, and `Link` children | The document text |
| `markdown_edit` | As `text_field(..).multiline()` in the source pane | `TextInput` beside a `Document` | As `text_field` |
| `code_edit` | As `text_field(..).multiline()`; your `.key_handler()` runs first | `TextInput` | "Code editor" |
| `mermaid` | none | `Figure` | "Mermaid" plus the diagram kind, for example "Mermaid graph"; the source is the description |
| `diff` | Enter or Space on the layout buttons | `List`, rows `ListItem` | `.path()`, else "Diff" |

On a terminal, `code_edit` uses one row per line and shows the caret only while the editor has focus.

## Terminal notes

- `mermaid` and `graphviz` render to monospace text in a terminal, and `math` to Unicode text.
- A markdown image uses the terminal image rules. Without `image-decode`, it paints as `[alt]` text.
- A link in `markdown` or `html` becomes an OSC 8 hyperlink when the terminal supports hyperlinks.
- `diff` in `DiffView::Split` needs about 80 columns to show both sides. Use `DiffView::Unified` in a narrow terminal.

## Example

Run it with `cargo run -p sigil-ui --example widget_documents`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_documents.rs}}
```
