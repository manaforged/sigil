# Documents

Parsers and viewers for structured text: markdown, HTML, source code, diffs, and diagrams. `markdown`, `html`, and `mermaid` bound their input. A source over 1 MiB renders as a "Source is larger than the 1 MiB limit." paragraph instead of parsing; `diff` parses unbounded.

## markdown

`markdown(Signal<String>) -> NodeBuilder`. Parses the signal into blocks inside a `derived`, so the document re-renders only when the parsed tree changes. The root is `Role::Document`, a vertical column.

Supported blocks: headings (`#`–`###`), paragraphs, `-`/`*` bullet lists (no ordered lists), fenced code, pipe tables (rendered as a monospace grid), `![alt](src)` images, and three Sigil-aware fences: ` ```mermaid ` renders through `mermaid()`, ` ```chart ` parses an optional title plus `label: value` rows into a `chart()`, and ` ```xychart ` reads the mermaid xychart form (`title`, `x-axis`, `bar`/`line` rows). A chart fence that fails to parse falls back to the mermaid path. Inline spans: `**bold**`, `` `code` ``, and `[text](href)` links. `*italic*` parses but currently renders as plain text. This is a deliberately bounded subset. Read the parser before relying on an edge case.

Images resolve `src` only as a `data:` base64 URI (SVG excluded). The widget never reads the filesystem or fetches a URL, so untrusted text cannot make it load a file. Images display at most 480×384 px. Decoding needs the `image-decode` feature. Without it, or on failure, the image renders as `[alt]` text.

`markdown_inline(&str) -> TextBuilder` renders one paragraph of inline spans as a single wrapped text node. Code spans keep their background, and the caller's `Paint` methods style the rest of the paragraph.

`markdown_edit(Signal<String>) -> MarkdownEditBuilder` is the editor split: `text_edit(source)` beside `markdown(source)` inside a horizontal `splitter`, each pane starting at 360 px. The preview follows the signal live. `.disabled(Signal<bool>)` disables the editor pane.

## html

`html(Signal<String>) -> NodeBuilder`. Parses a bounded HTML subset into the same `Block` tree as markdown and renders through the same `document` view: headings, paragraphs, lists, code, tables, images. `<script>` and `<style>` blocks are stripped entirely; unknown tags degrade to their text content. It is a viewer for trusted markup, not a browser engine.

## code_edit

`code_edit(lines, caret) -> CodeEditBuilder` where `lines` is `Signal<Vec<String>>` and `caret` is `Signal<(usize, usize)>` as `(row, column)`. A column is a grapheme cluster index, the same unit `text_edit` uses, so the caret and selection never split a grapheme. `HlSpan` and `DiagSpan` columns use the same unit. A line-oriented code editor that renders only the rows inside the scroll viewport. `first_row` derives the visible window from `scroll_y`, and an effect follows the caret so typing scrolls.

- `.highlights(Signal<Vec<Vec<HlSpan>>>)`: per-line syntax spans; `HlSpan { start, end, token }` maps a column range to a token class.
- `.selection(Signal<Option<(usize, usize)>>)`: selection anchor; the range from anchor to caret paints selected.
- `.gutter(Signal<Vec<Option<ChangeKind>>>)`: per-line `Added` / `Modified` / `Deleted` bars in the line-number gutter (sized to the line count: `18 + digits × 7` px).
- `.diagnostics(Signal<Vec<DiagSpan>>)`: `DiagSpan { line, start, end, severity }` tints a line range by `DiagSeverity` (Hint/Info → info, Warning → warn, Error → error).
- `.key(Fn(&mut KeyEvent))`: a key handler that runs before the built-in editing keys. Call `event.stop_propagation()` to replace a built-in binding. Installing one traps Tab for the handler instead of moving focus.
- `.row_height(px)`: defaults to 18 px, or one cell on a terminal.
- `.disabled(Signal<bool>)`: ignores keys, removes the editor from the focus order, and paints it at the disabled opacity.

The widget edits the `lines`, `caret`, and `.selection()` signals through the `text_edit` controller: typing, Enter, Backspace, Delete, arrow keys, Home, End, Page Up, Page Down, Shift selection, word motion, clipboard, undo and redo, and IME commit.

## mermaid

`mermaid(source: impl Into<String>) -> NodeBuilder`. Renders a diagram into a monospace text canvas (`font-mono text-sm` lines), not vectors. Supported heads: `graph`/`flowchart` (and `stateDiagram` / `stateDiagram-v2`), `sequenceDiagram`, and `pie`. Anything else renders the raw source inside a drawn box frame rather than failing.

Input is bounded to 256 lines of 512 chars and output to 512 lines of 1024 chars.

## diff

`diff(patch: impl Reactive<String>) -> DiffBuilder`. Parses a unified diff and renders it as split (side-by-side) or unified rows with a header showing the path and `+added`/`−removed` counts.

- `.path(text)`: header label; falls back to the patch's own paths.
- `.status(text)`: extra header label (e.g. "modified").
- `.view(DiffView::Split | DiffView::Unified)`: default Split.
- `.change(Fn(DiffView))`: makes the header's split/unified buttons live; the callback receives the picked layout so you can rebuild.
- `.max_rows(n)`: truncates rendered rows.

## States, keys, and accessibility

The visual catalog captures `code_edit` and `markdown_edit` in default, hover, focused, and disabled. `markdown`, `html`, `mermaid`, and `diff` are static views.

| Widget | Keys | Role | Name source |
| --- | --- | --- | --- |
| `markdown`, `html` | none | `Document`, with `Heading`, `Paragraph`, `List`, `ListItem`, `Code`, and `Link` children | The document text |
| `markdown_edit` | As `text_edit` in the source pane | `TextInput` beside a `Document` | As `text_edit` |
| `code_edit` | As `text_edit`; your `.key()` handler runs first | `TextInput` | "Code editor" |
| `mermaid` | none | `Figure` | "Mermaid" plus the diagram kind, for example "Mermaid graph"; the source is the description |
| `diff` | Enter or Space on the layout buttons | `List`, rows `ListItem` | `.path()`, else "Diff" |

On a terminal, `code_edit` uses one row per line and shows the caret only while the editor has focus.

## Terminal notes

- `mermaid` renders to monospace text, so it reads the same in a terminal.
- A markdown image uses the terminal image rules. Without `image-decode`, it paints as `[alt]` text.
- A link in `markdown` or `html` becomes an OSC 8 hyperlink when the terminal supports hyperlinks.
- `diff` in `DiffView::Split` needs about 80 columns to show both sides. Use `DiffView::Unified` in a narrow terminal.

## Example

Run it with `cargo run -p sigil-ui --example widget_documents`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_documents.rs}}
```
