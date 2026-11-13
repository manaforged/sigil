# Quickstart

Sigil is a UI library in Rust for desktop windows, terminals, and the web. State is a signal graph. Layout, text shaping, and the wgpu renderer are owned code. AccessKit, IME, typed styling, and the widget catalog ship in the crate. You write plain Rust against one import.

```rust,no_run
use sigil::prelude::*;
```

## Install

Add the dependency from [Install](https://github.com/manaforged/sigil#install).

The package is `sigil-ui` because the `sigil` name is taken on crates.io. The library is `sigil`, so imports read `sigil::`. [Terminal host](terminal.md) covers that host.

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
        .padding(Space::Xxl)
        .background(Token::Surface)
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("Increment").primary().activate(move || count.update(|n| *n += 1)))
}
```

The moving parts:

- `sigil::run(config, app_tree)` opens a window and drives the frame loop. The config is a title, such as `"Counter"`, or an `AppConfig` that also sets the size or the host. `app_tree` builds the node tree once.
- `signal(0)` creates a reactive cell. Signals are `Copy`; pass them into closures freely.
- `text(move || ...)` takes a closure. The closure re-runs when a signal it reads changes, and the text re-renders. `text` also takes a `&str`, a `String`, or a signal. You do not subscribe or schedule a repaint.
- The root fills the window. It needs no size of its own.
- `.activate(move || ...)` is an event handler: a plain closure that writes signals. `count.update(...)` changes the value in place; `count.get()` reads with subscription and `count.peek()` without.

A component is a function that returns `impl IntoNode`; signals it creates belong to it. `run` takes the root component: `run("Go", || button("Go"))`.

## Lists and conditions

`for_each` renders one row per item and keeps a row's node while its key stays. `show` swaps two branches on a condition.

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

`.autofocus()` on a node or a `text_field` gives it keyboard focus when it mounts.

## Three hosts

The tree is the same. The host changes.

| You want | You call |
|---|---|
| A desktop window | `sigil::run(config, tree)` (`hello-counter`) |
| Your own loop | `Host` inside `Owner::scope` (`hello-host`) |
| A character grid | `sigil::run(config.terminal(), tree)` (feature `terminal`, `hello-terminal`) |

`signal` panics outside `Owner::scope`. `sigil::run` opens that scope. An embed host that uses signals must call it.

`Host::scene` lays out the tree and returns the `CommandScene`. After layout, `Host::surfaces` lists `surface(key)` holes. You place a native view. The `sigil-ui` README covers the embed details.

## Styling

Styling is typed Rust. Paint goes through the `Paint` methods. Layout goes through the layout setters that every builder has, such as `padding`, `gap`, and `width`. A misspelt name does not compile.

```rust,no_run
# use sigil::prelude::*;
# let _ = {
container()
    .padding(Space::Lg)
    .gap(Space::Sm)
    .background(Token::Overlay)
    .border_width(1)
    .border_color(Token::OverlayBorder)
    .radius(Radius::Md)
    .shadow(Elevation::Md)
    .child(text("Title").text_role(TextRole::Title).text_color(Token::OnSurface))
    .child(text("12:30").font_family(FontFamily::Mono).text_color(Token::OnSurfaceMuted))
# };
```

`Paint` has these methods:

| Method | Sets |
| --- | --- |
| `background` | Background: a color, or a `Background::linear`, `radial`, or `conic` gradient; a later `background` replaces the earlier one |
| `text_color` | Text color |
| `border_color`, `border_width`, `border_top`, `border_right`, `border_bottom`, `border_left` | Border color and width in px |
| `radius(Radius)` | Corner radius |
| `shadow(Elevation)` | Drop shadow |
| `opacity(f32)` | Opacity, 0.0 to 1.0 |
| `font_size`, `font_weight`, `font_italic`, `font_family(FontFamily)` | Font |
| `line_height(px)`, `letter_spacing(em)`, `tabular_nums` | Text metrics: line height in pixels, letter spacing in ems |
| `underline`, `line_through` | Text decoration |
| `transition(Transition)` | Style transitions, as in Tailwind; `Transition::new()`, `all()`, `colors()`, `opacity()`, and `transform()` choose the properties, and `duration`, `ease`, and `delay` set the timing |
| `animate(Keyframes)` | Keyframe animation; `Keyframes::spin()`, `ping()`, `pulse()`, `bounce()` are the Tailwind ones |

`class("...")` on any builder sets the same styles from Tailwind classes, such as `class("bg-linear-to-r from-accent to-surface")`.

Padding, margin, and gap take a `Space` step such as `Space::Md`; width and height take a number of pixels or a `Length`. The layout setters set padding, margin, gap, size, flex, and grid. `row()` and `column()` are flex containers with the default gap.

### Typed theme tokens

`Token` names every theme color. `background`, `text_color`, and `border_color` take a `Token` or a `Color`. Theme tokens come from the active theme. Dark mode is a theme swap, not a restyle.

| Group | Tokens |
| --- | --- |
| Accent | `Accent`, `AccentHover`, `OnAccent`, `OnSolid` |
| Surfaces | `Surface`, `SurfaceHover`, `SurfacePressed`, `SurfacePressedStrong`, `SurfaceSubtle`, `SurfaceInset`, `Field` |
| Text | `OnSurface`, `OnSurfaceMuted` |
| Borders | `Border`, `BorderStrong` |
| Focus ring | `Ring` |
| Overlays | `Overlay`, `OverlayBorder`, `Scrim` |
| Scales | `Gray1`..`Gray12`, `Accent1`..`Accent12`, `Success1`.., `Warning1`.., `Error1`.., `Info1`.. |

`Token::color` returns a `ColorAtom`. `Theme::color(token)` resolves a token to a color. Widgets resolve tokens when they paint, so a theme swap recolours a built tree.

Shared metrics:

| Item | API | Value |
| --- | --- | --- |
| Spacing step | `space(n)` | `n` × 4 px |
| Radius family | `Radius::{Sm, Base, Md, Lg, Full}` | 2, 4, 6, 8, 9999 px; 0 on a character grid |
| Border width | `BORDER_WIDTH` | 1 px |
| Focus ring | `Widget::focus_ring` default | 2 px wide, 2 px offset, color `Ring` |
| Overlay | `Token::Overlay`, `Token::OverlayBorder`, `Metrics::overlay_padding` | fill, border, `Elevation::Md`, 4 px |
| Disabled | opacity | 50 % |
| Icon glyph | `Metrics::icon_size` | 20 px font size; one cell on a character grid |
| Control hover | `SurfaceHover` fill, `SurfacePressed` while pressed | checkbox, toggle, radio row, slider |
| Controls | `Metrics::current()` | `control_height`, `control_padding`, `control_gap` per `Size` |

A radius reads the theme at paint time. `Metrics::current()` returns the host metrics. On a character grid, control heights, padding, and gaps are one cell.

A theme can also live in a TOML file: `base = "dark"` (or `"light"`, `"console_dark"`) plus a `[tokens]` table of hex colors (`accent = "#7c3aed"`). `AppConfig::theme_file(path)` loads the file at launch in the window and on the terminal. Then it starts one `watch_theme` watcher. If the watcher cannot start, the theme still loads. Edits re-apply on the next frame, and writes in quick succession coalesce into one reload. For manual control, `sigil::app::watch_theme(path)` returns a `ThemeWatcher` guard that re-parses the file and calls `Theme::apply` on the UI thread until dropped; call it inside the app tree so the reload can reach the UI thread.

## Interaction

Every node takes interaction styling. `.hover()`, `.pressed()`, `.focus()`, `.disabled()`, and `.activate()` work on a plain container:

```rust,no_run
# use sigil::prelude::*;
# let _ = {
container()
    .width(328.0)
    .height(304.0)
    .padding(Space::Xxl)
    .background(Token::Accent)
    .radius(Radius::Md)
    .hover(|s| s.background(Token::AccentHover))
# };
```

The node owns the hit testing and the state, so a hover background needs no signal of your own. `button` already defines `.hover()`, `.focus()`, `.pressed()`, and `.disabled_paint()`.

## Where to go next

- Widgets: `button`, `text_field`, `text_edit`, `modal`, `virtual_list`, `table`, `card`, `avatar`, `sidebar`. Alerts are `sigil::app::dialog::message` / `confirm`; they return a signal and do not block.
- Architecture: [Overview](arch/overview.md), [State](arch/state.md), [Painting](arch/painting.md), [Platform](arch/platform.md), [Modules](arch/modules.md).
- The runnable widget demos: `crates/sigil/examples`.
- The terminal host: [Terminal host](terminal.md).

Build the book with `mdbook serve docs`.
