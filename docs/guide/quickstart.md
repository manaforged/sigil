# Quickstart

Sigil is a desktop UI library in Rust. State is a signal graph. Layout,
text shaping, and the wgpu renderer are owned code. AccessKit, IME,
utility classes, and the widget catalog ship in the crate. You write
plain Rust against one import.

```rust,ignore
use sigil::prelude::*;
```

## Install

```bash
cargo add sigil-ui
```

The package is `sigil-ui` because the `sigil` name is taken on
crates.io. The library is `sigil`, so imports read `sigil::`.

For a terminal-only program, turn off the default features:

```bash
cargo add sigil-ui --no-default-features --features terminal
```

[Terminal host](terminal.md) covers that host.

## The smallest app

A complete reactive counter (example `hello-counter`):

```rust,ignore
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - hello-counter", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label_text = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(480.0, 240.0).gap(16.0))
            .class(classes!("p-8 bg-surface"))
            .child(text(label_text).class(classes!("text-on-surface")))
            .child(button("Increment").primary().activate(move || {
                count.set(count.peek() + 1);
            })),
    )
}
```

The moving parts:

- `sigil::run((title, width, height), app_tree)` opens a window and
  drives the frame loop. `app_tree` builds the node tree once.
- `signal(0)` creates a reactive cell. Signals are `Copy`; pass them
  into closures freely.
- `derived(...)` computes a value from signals and re-evaluates when
  they change. `text(label_text)` re-renders when the count changes.
  You do not subscribe or schedule a repaint.
- `.activate(move || ...)` is an event handler: a plain closure that
  writes signals. `count.peek()` reads without subscribing the
  enclosing effect; `count.get()` reads with subscription.

Widget helpers implement `IntoNode`; type a layout helper as
`impl IntoNode`. `run` takes a closure, `text`, `container`, or
`root(button("Go"))`.

## Three hosts

The tree is the same. The host changes.

| You want | You call |
|---|---|
| A desktop window | `sigil::run(config, tree)` (`hello-counter`) |
| Your own loop | `Host` inside `Runtime::scope` (`hello-host`) |
| A character grid | `sigil::run(config.terminal(), tree)` (feature `terminal`, `hello-terminal`) |

`signal` panics outside `Runtime::scope`. `sigil::run` opens that scope. An embed host that uses signals must call it.

`Host::scene` lays out the tree and returns the `CommandScene`.
After layout, `Host::surfaces` lists `surface(key)`
holes. You place a native view. The `sigil-ui` README covers the embed
details.

## Styling

`classes!("p-8 bg-surface rounded-md")` is the default styling path.
The tokens are Tailwind-shaped and parsed at runtime; a typo warns and
is ignored. Spacing tokens are 4px steps (`p-4` = 16px, `w-4` = 16px).
`opacity-50` is a percent. Theme tokens (`bg-surface`, `text-on-surface`,
`bg-accent`) come from the active theme. Dark mode is a theme swap, not
a restyle.

### Typed theme tokens

`Token` names every theme colour. An unknown name does not compile.
Class strings still parse token names at runtime.

```rust,ignore
use sigil::style::{BORDER_WIDTH, Radius, StyleAtom, Token};

container()
    .class("p-4")
    .class([
        Token::Overlay.bg(),
        Token::OverlayBorder.border(),
        StyleAtom::BorderWidth(BORDER_WIDTH),
        Radius::Md.atom(),
    ])
```

| Group | Tokens |
| --- | --- |
| Accent | `Accent`, `AccentHover`, `OnAccent`, `OnSolid` |
| Surfaces | `Surface`, `SurfaceHover`, `SurfacePressed`, `SurfacePressedStrong`, `SurfaceSubtle`, `SurfaceInset`, `Field` |
| Text | `OnSurface`, `OnSurfaceMuted` |
| Borders | `Border`, `BorderStrong` |
| Focus ring | `Ring` |
| Overlays | `Overlay`, `OverlayBorder`, `Scrim` |
| Scales | `Gray1`..`Gray12`, `Accent1`..`Accent12`, `Success1`.., `Warn1`.., `Error1`.., `Info1`.. |

`Token::bg`, `Token::text`, and `Token::border` return a `StyleAtom`.
`Token::color` returns a `ColorAtom`. `Theme::color(token)` resolves a
token to a colour. Widgets resolve tokens when they paint, so a theme
swap recolours a built tree.

Shared metrics:

| Item | API | Value |
| --- | --- | --- |
| Spacing | `space(n)` | `n` × 4 px |
| Radius family | `Radius::{Sm, Md, Lg, Full}.atom()` | 2, 6, 8, 9999 px; 0 on a character grid |
| Border width | `BORDER_WIDTH` | 1 px |
| Focus ring | `FocusRing` default | 2 px wide, 2 px offset, colour `Ring` |
| Overlay | `Token::Overlay`, `Token::OverlayBorder`, `OVERLAY_SHADOW`, `Metrics::overlay_padding` | fill, border, `Elevation::Md`, 4 px |
| Disabled | `DISABLED_OPACITY` | 50 % |
| Icon glyph | `ICON_SIZE`, `Metrics::icon_size` | 20 px font size; one cell on a character grid |
| Control hover | `SurfaceHover` fill, `SurfacePressed` while pressed | checkbox, toggle, radio row, slider, `interactive` |
| Controls | `Metrics::current()` | `control_height`, `padding`, `gap` per `Size` |

A radius atom reads the theme at paint time. `Metrics::current()`
returns the host metrics. On a character grid, control heights,
padding, and gaps are one cell.

A theme can also live in a TOML file: `base = "dark"` (or `"light"`,
`"console_dark"`) plus a `[tokens]` table of hex colors
(`accent = "#7c3aed"`). `AppConfig::theme_file(path)` loads the file at
launch in the window and on the terminal. Then it starts one
`watch_theme` watcher. If the watcher cannot start, the theme still
loads. Edits re-apply on the next frame, and writes in quick succession
coalesce into one reload. For manual control,
`sigil::watch_theme(path)` returns a `ThemeWatcher` guard that re-parses
the file and calls `set_theme` on the UI thread until dropped; call it
inside the app tree so the reload can reach the UI thread.

## Interaction

Static styling works on any node. Interaction styling goes through
`interactive()`, the opt-in wrapper that gives plain containers hover,
press, focus, and disabled semantics:

```rust,ignore
interactive(
    container()
        .layout(LayoutStyle::block().fixed_size(328.0, 304.0))
        .class(classes!("p-8 bg-accent rounded-md")),
)
.hover(|s| s.bg("accent-hover"))
```

`interactive()` owns the hit testing and the state, so a hover
background needs no signal of your own. `button` already defines
`.hover()`, `.focus()`, `.pressed()`, and `.disabled_style()` closures
out of the box.

## Where to go next

- Widgets: `button`, `icon_button`, `text_field`, `text_edit`,
  `modal` / `sheet`, `virtual_list`, `table`, `card`, `avatar`,
  `sidebar`. Native alert is `sigil::dialog::message` / `confirm`.
- Architecture: [Overview](arch/overview.md),
  [State](arch/state.md), [Painting](arch/painting.md),
  [Platform](arch/platform.md), [Modules](arch/modules.md).
- The runnable widget demos: `crates/sigil/examples`.
- The terminal host: [Terminal host](terminal.md).

Build the book with `mdbook serve docs`.
