# Styling

Sigil's style system is three layers over one atom model: **utility
classes** for the 80% case, a **struct API** for typed authoring, and
**fluent traits** for per-method chaining — all resolving into the same
`StyleAtom` vocabulary with one deterministic cascade. This chapter is
the whole system.

## Utility classes (the default)

```rust,ignore
container().class(classes!("p-4 bg-accent rounded-md text-on-surface"))
```

- **Compile-time checked** via the `classes!` macro (exported from
  `sigil::prelude`). An unknown token fails the build with a
  nearest-match suggestion — no silent typos.
- Dynamic strings still work at runtime (`.class(dynamic)`) and warn on
  unknown tokens instead of failing the build; `try_class` is the
  fallible dynamic path.
- A plain `container().class(...)` rejects state prefixes (`hover:`,
  `focus:`, `active:`, `disabled:`). The container has no interaction
  channel. Use `interactive(container()).class("bg-surface hover:bg-accent-9")`
  for a class-string hover, or a widget `.hover(|s| …)` closure. See
  `interactive()` below.
- `sm:` / `md:` / `lg:` prefixes are **rejected loudly** — never
  silently ignored.

**Scale conventions** (one rule per property, named at every entry
point):

- Spacing/size tokens are **Tailwind 4px steps**: `p-4` = 16px,
  `w-4` = 16px, `gap-2` = 8px. The struct/fluent APIs take **raw
  logical px** instead (`.padding(16)`, `.w(64.0)`).
- `opacity-N` is a **percent** (`opacity-50`); `.opacity(0.5)` is the
  CSS-canonical **fraction** everywhere in typed APIs.
- `shadow-*` tokens paint elevation. They are not rejected.

The token vocabulary (all also in the `sigil::prelude` docstring):
spacing `p/m/gap-N` (+per-side, `gap-x-N`, `gap-y-N`), sizing
`w/h/min-w/max-w/min-h/max-h-N`, `*-full`, `*-auto`, display
`flex/block/hidden`, direction `flex-row/col(-reverse)`,
`flex-wrap/nowrap`, `grow(-0)`, `shrink(-0)`, alignment
`items-start/center/end/stretch`, `justify-start/center/end/between/around/evenly`,
overflow `overflow-(x-|y-)visible/hidden/scroll/auto`, radius
`rounded(-sm/-md/-lg/-full)`, borders `border`, `border-N`,
`border-<token>`, colors `bg-<name>`, `text-<name>`, text sizes
`text-xs…xl`, weight/family `font-*`, `leading-N`, `opacity-N`.

## The cascade (deterministic, no specificity algorithm)

`LayoutStyle` padding on the node is the box. Class `p-N` and
`Style::padding` fill that slot only when the box is still zero.

Later wins per property, in this order:

1. Theme tokens (built-in + runtime).
2. Class strings / static atoms.
3. Owned (`Style` struct) atoms.
4. Reactive slices (`style_reactive`, `class_when`, widget variants) —
   **multiple reactive sources compose in install order** (T0): hover
   and selection styling on the same node no longer clobber.
5. Per-instance state overlays (`.hover()` / `.focus()` / `.pressed()` /
   `.disabled_style()` closures) — always last, so user atoms beat
   widget-baked atoms per property.

Text color and typography inherit down the tree (matching the CSS
inherited set); everything else is per-node.

## Struct API

Typed, no strings, one encoding per property:

```rust,ignore
Style::new()
    .bg("accent")
    .radius(6)
    .opacity(0.5)
container()
    .layout(LayoutStyle::block().padding(16.0))
    .style(style)
```

Padding belongs on the box (`LayoutStyle` / widget `.padding(px)`).
`Style::padding` is the same atom as `p-N`, not a second inset API.

## Fluent surface (`Styled`)

Chain per-method calls on any builder — `.bg("surface").px(16).py(8)`
— with the state closures for interaction:

```rust,ignore
button("Run")
    .primary()
    .hover(|s| s.bg("accent-9").opacity(0.9))
    .disabled_style(|s| s.opacity(0.4))
```

The closures record per-slot overlays (`rest/hover/focus/pressed/
disabled`). On interactive widgets (button, checkbox, dropdown, slider,
tabs, text_field, text_edit) they paint. On display-only widgets and
plain containers they are **compile errors by design** — use
`interactive()` (below) for containers. `.disabled_style` is the
style-closure name; `.disabled(signal)` stays the behavioral switch.

## `interactive()` — containers with interaction

Plain containers never become interactive by accident (hittability and
focus change occlusion and tab order). Opt in deliberately:

```rust,ignore
interactive(container().class(classes!("p-4 rounded-md bg-surface")))
    .focusable(true)
    .hover(|s| s.bg("surface-hover"))
    .pressed(|s| s.bg("surface-pressed"))
    .disabled_signal(disabled)
```

Class-string hover is the same channel:

```rust,ignore
interactive(container())
    .class("p-4 rounded-md bg-surface hover:bg-accent-9")
```

A Linear-style issue row is that sentence with flex and padding, and
no `LayoutStyle` on the row. Run `cargo run -p linear-row`.

Hover does **not** stop propagation — ancestor hover survives nesting.

## Themes, tokens, and tone × variant

- **12-step scales** per role (`gray`, `accent`, `error`, `warn`,
  `success`, `info`): `bg-error-9` addresses a scale step; semantic
  aliases (`bg-surface`, `bg-accent-hover`) sit on top.
- **Tone × variant** re-skins a widget through shared
  `SOLID_VARIANT`/`SOFT_VARIANT` tables with WCAG-aware foregrounds:
  `.tone(Tone::Error)`, `.solid()`/`.soft()`, or `.primary()` /
  `.destructive()` sugar on button. Adding a tone is a two-line change.
- **`.theme(Theme)`** scopes a theme to a subtree (nearest ancestor
  wins). Presets: `Theme::default()`, `dark()`, `console_dark()`.
- **Runtime tokens** extend any theme at runtime
  (`ThemeBuilder::token`, or a watched theme file — see Hot reload).

## State styling recap

| You want | Use |
|---|---|
| Static utility style | `classes!("p-4 bg-accent")` |
| Typed per-property | `Style` struct or fluent `.px(16)` |
| Value-driven class | `.class_when(signal, "bg-accent")` |
| Arbitrary reactive atoms | `.style_reactive(...)` |
| Hover/focus/pressed/disabled | widget closures, `interactive()`, or `interactive().class("hover:bg-…")` |

## Hot reload (dev loop)

`AppConfig::theme_file("theme.toml")` watches a TOML file and repaints
on save — edit tokens, see them live, no rebuild:

```toml
base = "dark"
[tokens]
accent = "#7c3aed"
surface-hover = "#f5f5f5"
```
