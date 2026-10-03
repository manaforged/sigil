# Quickstart

Sigil is a desktop UI library in Rust. State is a signal graph. Layout,
text shaping, and the wgpu renderer are owned code. AccessKit, IME,
typed styling, and the widget catalog ship in the crate. You write
plain Rust against one import.

```rust,no_run
use sigil::prelude::*;
```

## Install

Sigil is not on crates.io yet. Add the Git dependency from
[Install](https://github.com/manaforged/sigil#install).

The package is `sigil-ui` because the `sigil` name is taken on
crates.io. The library is `sigil`, so imports read `sigil::`.
[Terminal host](terminal.md) covers that host.

## The smallest app

A complete reactive counter (example `hello-counter`):

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - hello-counter").size(480, 240), app_tree)
}

fn app_tree() -> impl IntoNode {
    let count = signal(0_i32);
    column()
        .layout(LayoutStyle::default().padding(32.0))
        .bg(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("Increment").primary().activate(move || count.update(|n| *n += 1)))
}
```

The moving parts:

- `sigil::run(config, app_tree)` opens a window and drives the frame
  loop. The config is a title, such as `"Counter"`, or an `AppConfig`
  that also sets the size or the host. `app_tree` builds the node tree
  once.
- `signal(0)` creates a reactive cell. Signals are `Copy`; pass them
  into closures freely.
- `text(move || ...)` takes a closure. The closure re-runs when a
  signal it reads changes, and the text re-renders. `text` also takes a
  `&str`, a `String`, or a signal. You do not subscribe or schedule a
  repaint.
- The root fills the window. It needs no size of its own.
- `.activate(move || ...)` is an event handler: a plain closure that
  writes signals. `count.update(...)` changes the value in place;
  `count.get()` reads with subscription and `count.peek()` without.

A component is a function that returns `impl IntoNode`; signals it
creates belong to it. `run` takes the root component:
`run("Go", || button("Go"))`.

## Lists and conditions

`for_each` renders one row per item and keeps a row's node while its
key stays. `show` swaps two branches on a condition.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
let names = signal(vec![String::from("Ada"), String::from("Lin")]);
let empty = move || names.with(Vec::is_empty);
container()
    .child(for_each(names, String::clone, |name: &String| text(name.clone())))
    .child(show(empty, || text("No names"), || fragment()))
# };
```

`.autofocus()` on a node or a `text_field` gives it keyboard focus when
it mounts.

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

Styling is typed Rust. Paint goes through the `Paint` methods. Layout
goes through `LayoutStyle`. A misspelt name does not compile.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
container()
    .layout(LayoutStyle::default().padding(16.0).gap(8.0))
    .bg(Token::Overlay)
    .border_w(1)
    .border_color(Token::OverlayBorder)
    .rounded(Radius::Md)
    .shadow(Elevation::Md)
    .child(text("Title").font_size(18.0).font_weight(600).text_color(Token::OnSurface))
    .child(text("12:30").font_family(FontFamily::Mono).text_color(Token::OnSurfaceMuted))
# };
```

`Paint` has these methods:

| Method | Sets |
| --- | --- |
| `bg`, `background_gradient` | Background colour or gradient |
| `text_color` | Text colour |
| `border_color`, `border_w`, `border_top`, `border_right`, `border_bottom`, `border_left` | Border colour and width in px |
| `rounded(Radius)` | Corner radius |
| `shadow(Elevation)` | Drop shadow |
| `opacity(f32)` | Opacity, 0.0 to 1.0 |
| `font_size`, `font_weight`, `font_italic`, `font_family(FontFamily)` | Font |
| `line_height_px`, `letter_spacing_em`, `tabular_nums` | Text metrics; each name carries its unit |
| `underline`, `line_through` | Text decoration |
| `transition`, `duration` | Style transitions |

Spacing and size numbers are pixels. `LayoutStyle` sets padding,
margin, gap, size, flex, and grid. `row()` and `column()` are flex
containers with the default gap. Put all layout for one node in one
`LayoutStyle` chain.

### Typed theme tokens

`Token` names every theme colour. `bg`, `text_color`, and
`border_color` take a `Token` or an `Rgba8`. Theme tokens come from the
active theme. Dark mode is a theme swap, not a restyle.

| Group | Tokens |
| --- | --- |
| Accent | `Accent`, `AccentHover`, `OnAccent`, `OnSolid` |
| Surfaces | `Surface`, `SurfaceHover`, `SurfacePressed`, `SurfacePressedStrong`, `SurfaceSubtle`, `SurfaceInset`, `Field` |
| Text | `OnSurface`, `OnSurfaceMuted` |
| Borders | `Border`, `BorderStrong` |
| Focus ring | `Ring` |
| Overlays | `Overlay`, `OverlayBorder`, `Scrim` |
| Scales | `Gray1`..`Gray12`, `Accent1`..`Accent12`, `Success1`.., `Warn1`.., `Error1`.., `Info1`.. |

`Token::color` returns a `ColorAtom`. `Theme::color(token)` resolves a
token to a colour. Widgets resolve tokens when they paint, so a theme
swap recolours a built tree.

Shared metrics:

| Item | API | Value |
| --- | --- | --- |
| Spacing step | `space(n)` | `n` × 4 px |
| Radius family | `Radius::{Sm, Base, Md, Lg, Full}` | 2, 4, 6, 8, 9999 px; 0 on a character grid |
| Border width | `BORDER_WIDTH` | 1 px |
| Focus ring | `FocusRing` default | 2 px wide, 2 px offset, colour `Ring` |
| Overlay | `Token::Overlay`, `Token::OverlayBorder`, `OVERLAY_SHADOW`, `Metrics::overlay_padding` | fill, border, `Elevation::Md`, 4 px |
| Disabled | `DISABLED_OPACITY` | 50 % |
| Icon glyph | `ICON_SIZE`, `Metrics::icon_size` | 20 px font size; one cell on a character grid |
| Control hover | `SurfaceHover` fill, `SurfacePressed` while pressed | checkbox, toggle, radio row, slider, `interactive` |
| Controls | `Metrics::current()` | `control_height`, `padding`, `gap` per `Size` |

A radius reads the theme at paint time. `Metrics::current()`
returns the host metrics. On a character grid, control heights,
padding, and gaps are one cell.

A theme can also live in a TOML file: `base = "dark"` (or `"light"`,
`"console_dark"`) plus a `[tokens]` table of hex colors
(`accent = "#7c3aed"`). `AppConfig::theme_file(path)` loads the file at
launch in the window and on the terminal. Then it starts one
`watch_theme` watcher. If the watcher cannot start, the theme still
loads. Edits re-apply on the next frame, and writes in quick succession
coalesce into one reload. For manual control,
`sigil::app::watch_theme(path)` returns a `ThemeWatcher` guard that re-parses
the file and calls `set_theme` on the UI thread until dropped; call it
inside the app tree so the reload can reach the UI thread.

## Interaction

Static styling works on any node. Interaction styling goes through
`interactive()`, the opt-in wrapper that gives plain containers hover,
press, focus, and disabled semantics:

```rust,no_run
# use sigil::prelude::*;
# let _ = {
interactive(
    container()
        .layout(LayoutStyle::block().fixed_size(328.0, 304.0).padding(32.0))
        .bg(Token::Accent)
        .rounded(Radius::Md),
)
.hover(|s| s.bg(Token::AccentHover))
# };
```

`interactive()` owns the hit testing and the state, so a hover
background needs no signal of your own. `button` already defines
`.hover()`, `.focus()`, `.pressed()`, and `.disabled_style()` closures
out of the box.

## Where to go next

- Widgets: `button`, `icon_button`, `text_field`, `text_edit`,
  `modal` / `sheet`, `virtual_list`, `table`, `card`, `avatar`,
  `sidebar`. Alerts are `sigil::app::dialog::message` / `confirm`; they return a
  signal and do not block.
- Architecture: [Overview](arch/overview.md),
  [State](arch/state.md), [Painting](arch/painting.md),
  [Platform](arch/platform.md), [Modules](arch/modules.md).
- The runnable widget demos: `crates/sigil/examples`.
- The terminal host: [Terminal host](terminal.md).

Build the book with `mdbook serve docs`.
