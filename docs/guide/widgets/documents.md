# Documents

Parsers and viewers for structured text: markdown, HTML, source code,
diffs, and diagrams. `markdown`, `html`, and `mermaid` bound their
input — a source over 1 MiB renders as a "Source is larger than the
1 MiB limit." paragraph instead of parsing; `diff` parses unbounded.

## markdown

`markdown(Signal<String>) -> NodeBuilder`. Parses the signal into
blocks inside a `derived_eq`, so the document re-renders only when the
parsed tree changes. The root is `Role::Document`, a vertical column.

Supported blocks: headings (`#`–`###`), paragraphs, `-`/`*` bullet
lists (no ordered lists), fenced code, pipe tables (rendered as a
monospace grid), `![alt](src)` images, and three Sigil-aware fences:
` ```mermaid ` renders through `mermaid()`, ` ```chart ` parses an
optional title plus `label: value` rows into a `chart()`, and
` ```xychart ` reads the mermaid xychart form (`title`, `x-axis`,
`bar`/`line` rows). A chart fence that fails to parse falls back to
the mermaid path.
Inline spans: `**bold**`, `` `code` ``, and `[text](href)` links.
`*italic*` parses but currently renders as plain text. This is a
deliberately bounded subset — read the parser before relying on an edge
case.

Images resolve `src` only as a `data:` base64 URI (SVG excluded).
The widget never reads the filesystem or fetches a URL, so untrusted
text cannot make it load a file. Images display at most 480×384 px.
Decoding needs the `image-decode` feature. Without it, or on failure,
the image renders as `[alt]` text.

`markdown_edit(Signal<String>) -> NodeBuilder` is the editor split:
`text_edit(source)` beside `markdown(source)` inside a horizontal
`splitter`, each pane starting at 360 px. The preview follows the
signal live.

## html

`html(Signal<String>) -> NodeBuilder`. Parses a bounded HTML subset
into the same `Block` tree as markdown and renders through the same
`document` view — headings, paragraphs, lists, code, tables, images.
`<script>` and `<style>` blocks are stripped entirely; unknown tags
degrade to their text content. It is a viewer for trusted markup, not
a browser engine.

## code_edit

`code_edit(lines, caret) -> CodeEditBuilder` where `lines` is
`Signal<Vec<String>>` and `caret` is `Signal<(usize, usize)>` as
`(row, column)`. A line-oriented code editor that renders only the
rows inside the scroll viewport — `first_row` derives the visible
window from `scroll_y`, and an effect follows the caret so typing
scrolls.

- `.highlights(Signal<Vec<Vec<HlSpan>>>)` — per-line syntax spans;
  `HlSpan { start, end, token }` maps a byte range to a token class.
- `.selection(Signal<Option<(usize, usize)>>)` — selection anchor; the
  range from anchor to caret paints selected.
- `.gutter(Signal<Vec<Option<ChangeKind>>>)` — per-line `Added` /
  `Modified` / `Deleted` bars in the line-number gutter (sized to the
  line count: `18 + digits × 7` px).
- `.diagnostics(Signal<Vec<DiagSpan>>)` — `DiagSpan { line, start,
  end, severity }` tints a line range by `DiagSeverity`
  (Hint/Info → info, Warning → warn, Error → error).
- `.key(Fn(&mut KeyEvent))` — a raw key handler for custom bindings;
  installing one traps Tab for the handler instead of moving focus.
- `.row_height(px)`.

The buffer is just `Vec<String>` — the widget edits through the signal
and the host controls where content comes from.

## mermaid

`mermaid(source: impl Into<String>) -> NodeBuilder`. Renders a diagram
into a monospace text canvas (`font-mono text-sm` lines), not vectors.
Supported heads: `graph`/`flowchart` (and `stateDiagram` /
`stateDiagram-v2`), `sequenceDiagram`, and `pie` —
`mermaid_diagram(source)` reports whether a source will parse as one.
Anything else renders the raw source inside a drawn box frame rather
than failing.

Input is bounded to 256 lines of 512 chars and output to 512 lines of
1024 chars. `mermaid_draw(source) -> Vec<String>` is public if you want
the text canvas itself.

## diff

`diff(patch: impl Into<String>) -> DiffBuilder`. Parses a unified diff
and renders it as split (side-by-side) or unified rows with a header
showing the path and `+added`/`−removed` counts.

- `.path(text)` — header label; falls back to the patch's own paths.
- `.status(text)` — extra header label (e.g. "modified").
- `.layout(DiffLayout::Split | DiffLayout::Unified)` — default Split.
- `.choose(Fn(DiffLayout))` — makes the header's split/unified buttons
  live; the callback receives the picked layout so you can rebuild.
- `.max_rows(n)` — truncates rendered rows.
