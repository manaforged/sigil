# GUI and TUI

One widget tree runs in a desktop window (GUI) and in a terminal (TUI). This page tells you what the two hosts share, what belongs to one host, and how to write code for one host without limiting the other.

## Choose the hosts when you build

Cargo features decide which hosts are in your binary.

| Your app | Dependency | What builds |
| --- | --- | --- |
| GUI only | `sigil-ui = { git = "https://github.com/manaforged/sigil" }` | The window host: wgpu, winit, and AccessKit. No crossterm. |
| TUI only | `sigil-ui = { git = "https://github.com/manaforged/sigil", default-features = false, features = ["terminal"] }` | The terminal host: crossterm. No wgpu and no winit. |
| Both | `sigil-ui = { git = "https://github.com/manaforged/sigil", features = ["terminal"] }` | Both hosts. You pick one each run. |

`sigil::run` opens a window by default. `AppConfig::terminal()` opens the terminal host instead. An app with both hosts picks one at start:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    let config = AppConfig::new("Notes");
    let config = if std::env::args().any(|arg| arg == "--tui") {
        config.terminal()
    } else {
        config.size(800, 600)
    };
    sigil::run(config, app_tree)
}

fn app_tree() -> impl IntoNode {
    text("Notes")
}
```

## What the hosts share

Both hosts run the same components, signals, layout, events, focus, accessibility, themes, and tests. A component that uses only these needs no host code. Write it once.

## What belongs to one host

### GUI only

- Native menus: `sigil::app::menu` (feature `menu`).
- The system tray: `sigil::app::tray` (feature `tray`).
- OS file dialogs (feature `native-dialog`). Feature `dialog` gives an in-app dialog on both hosts.
- File drops from the desktop.
- Shadows, blur, and gradients.
- `surface(key)` holes for an embed host, and the browser canvas.

### TUI only

- `sigil::terminal`: hooks, capabilities, cells, and images on the grid.
- Inline mode and `sigil::terminal::print_above`.
- The OSC 52 clipboard, which `run` installs.
- The `SIGIL_TUI_TRANSCRIPT` accessibility transcript.
- Box-drawing borders and cell colors.

A GUI-only API compiles out of a TUI-only build because its feature is off. You do not guard it at run time.

## Write the GUI the way you want

Paint the window with every tool it has: shadows, blur, gradients, rounded corners, images, and pixel sizes. The terminal host keeps what a cell can show and drops the rest: a shadow, a blur, or a gradient is omitted, a rounded box becomes rounded box-drawing glyphs, and an image goes through the terminal graphics protocol or sextant glyphs. The [Terminal host](terminal.md#scene-mapping) page lists each mapping. You do not add a terminal branch to keep a GUI style.

The window host paints the theme's `Surface` color behind the tree, so an area the tree leaves unpainted follows the theme. A transparent window skips it. The terminal host leaves unpainted cells to the terminal's own background.

Give text-sized lengths in `rows`, `cols`, or `em`, and give picture sizes in pixels. Then the terminal gets whole cells and the window gets text-height controls from the same code. See [Units](units.md).

## Branch when the screen differs

When one host needs a different screen, branch on `sigil::app::is_grid()` at the top of a component. It returns `true` on the terminal host and `false` on the window host, and it is already set when the component runs. The host does not change while the app runs, so the branch is fixed for the life of the node.

```rust
use sigil::app::is_grid;
use sigil::prelude::*;

fn inbox(selected: Signal<usize>) -> impl IntoNode {
    if is_grid() {
        column().child(text(move || format!("Message {}", selected.get())))
    } else {
        row()
            .child(text("Folders"))
            .child(text(move || format!("Message {}", selected.get())))
    }
}
```

Keep the state outside the branch, as `selected` is here. Both screens then share one copy of the behavior, and only the layout differs. Branch at the screen level. Do not branch inside small widgets: Sigil's own widgets already take the cell form on the grid.

## Split host code at compile time

Code that calls a GUI-only or TUI-only API goes behind your own feature, which turns on the Sigil feature:

```toml
[features]
default = ["gui"]
gui = ["sigil-ui/menu"]
tui = ["sigil-ui/terminal"]
```

```rust,ignore
#[cfg(feature = "gui")]
fn install_menu() {
    sigil::app::menu::set_menu_bar(app_menu());
}
```

A TUI-only build then has no menu code and no GPU stack, and a GUI-only build has no terminal code.

## Test both hosts

`sigil-kittest` drives the window host and compares PNG goldens; see [Testing](testing.md). The terminal host runs headless through `Host::cells` and `sigil::terminal::step`; see the [API map](../api.md). To check that a TUI-only build has no GPU stack:

```bash
cargo tree -p your-app --no-default-features --features tui | grep -E 'wgpu|winit'
```

The command prints nothing.
