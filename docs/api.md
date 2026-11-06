# Sigil API map

This page lists every item that `sigil` exports at the crate root and in `sigil::prelude`. Each item has a signature, a one-line purpose, and an example. Every example on this page compiles as a doctest of the `sigil` crate.

The package is `sigil-ui`. The library is `sigil`. The API can change between 0.x releases.

[Install](https://github.com/manaforged/sigil#install) gives the dependency line.

Application code imports the prelude:

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("App").terminal(), || text("Hello"))
}
# }
# fn main() {}
```

The default feature `desktop` gives the window host. The feature `terminal` gives the terminal host. A terminal-only program uses `default-features = false, features = ["terminal"]`. The [crate README](../crates/sigil/README.md#features) lists every Cargo feature. For task-oriented examples, see [Recipes](guide/recipes.md).

## Crate root

The public modules are `animation`, `app`, `clipboard`, `events`, `geometry`, `icons`, `profile`, `reactive`, `scene`, `style`, `tasks`, `terminal`, and `widgets`. Tree and layout types are at the crate root or in the prelude. Prefer the prelude and the functions in this table.

A type in a signature without its own row has no public path. A function returns it or a closure receives it, and its methods work without an import.

| Name | Signature | Purpose |
| --- | --- | --- |
| `geometry` | `mod geometry` | Points, rectangles, and sizes in logical and physical pixels. |
| `prelude` | `mod prelude` | The application import surface. |
| `animation` | `mod animation` | Easing, the animation clock, and reduced motion. |
| `app` | `mod app` | Application host, configuration, and theme files. |
| `clipboard` | `mod clipboard` | Clipboard trait and the installed clipboard. |
| `events` | `mod events` | Event routing, handlers, and focus. |
| `icons` | `mod icons` | Bundled icon glyphs (feature `scene`). |
| `profile` | `mod profile` | Frame timing counters. |
| `reactive` | `mod reactive` | Signals, effects, owners, and intervals. |
| `scene` | `mod scene` | The `CommandScene` display list. |
| `style` | `mod style` | Paint, themes, tokens, and metrics. |
| `tasks` | `mod tasks` | Async tasks and cross-thread dispatch. |
| `terminal` | `mod terminal` | The terminal host (feature `terminal`). On Unix, SIGTERM, SIGHUP, and SIGINT restore the terminal and return from `sigil::run`; the application owns process cleanup and exit status. |
| `widgets` | `mod widgets` | Widget builders. |

| `run` | `fn run<C: Into<AppConfig>, N: IntoNode>(config: C, app: impl Fn() -> N + 'static) -> AppResult` | Run the app component on the host that `config` selects. `app` is called once, inside the root's reactive owner, and again after a hot patch. |
| `request_exit` | `fn request_exit()` | Ask the running host to exit after this frame. |
| `dialog::FileDialog` | `FileDialog::new()`; `filter(name, &[ext])`, `directory(path)`, `file_name(name)`; `open_file()`, `open_files()`, `pick_folder()`, `save_file()` | Builder of a file dialog. Each pick returns a `Picked` signal that turns from `Pending` to `Ok(None)` on cancel or `Ok(Some(path))`. The in-app picker returns one path from `open_files()`. |
| `dialog::Picked` | `type Picked<T> = Signal<Status<Option<T>, Infallible>>` | Result of a file dialog. |
| `dialog::open_file`, `dialog::save_file`, `dialog::pick_folder` | `fn() -> Picked<PathBuf>` | `FileDialog::new()` with one pick. |
| `dialog::message`, `dialog::confirm` | `fn(title, body) -> Signal<Status<bool, Infallible>>` | An alert with OK, or OK and Cancel. Resolves `Ok(true)` on OK. |
| `dialog::mock` | `push_path`, `push_paths`, `push_alert`, `reset` (`cfg(test)` or feature `kittest`) | Queues the next dialog answers. A queued answer resolves the signal at once and opens no dialog. |
| `AppConfig` | `struct AppConfig`; `From<&str>`, `From<String>`; `AppConfig::new(title)`; host: `window()` (feature `desktop`), `terminal()` (feature `terminal`), `canvas(id)` (feature `desktop`); window: `size(w, h)`, and with feature `desktop` `min_size(w, h)`, `user_zoom(f32)`, `frameless(bool)`, `overlay(bool)`, `anchor(Anchor)`; `theme_file(path)`, `panic_policy(PanicPolicy)`, `quit_on_ctrl_c(bool)`, `frame_interval(Duration)` | Run configuration: title, size, host, theme file, web canvas. A title converts to a config with the defaults; `AppConfig::new(title)` starts the builder. `window()` selects the window host and `terminal()` the terminal host. Without a call, the host is the window when feature `desktop` is on, else the terminal. A host method exists only with its feature, so a build without that host does not compile the call. `canvas` names the web canvas element. `user_zoom` scales the UI; the default is 1.0. `frameless(true)` removes the window frame. `overlay(true)` opens a transparent accessory window; `anchor` places it on the screen. `theme_file` loads the theme from a file and reloads it when the file changes. `quit_on_ctrl_c` is true by default. `frame_interval` sets the wait between terminal host frames; the default is 1/60 s (60 Hz). |
| `AppResult` | `type AppResult<T> = Result<T, Error>` | Result of the application functions. |
| `Error` | `struct Error` | Opaque application error. |
| `Host` | `struct Host` | Embed host: build, route, lay out, and paint one tree. `set_hyperlinks(bool)` tells widgets whether the host can show hyperlinks. |
| `Host::reset_emit` | `fn reset_emit(&mut self)` | Clear the memo that incremental painting reuses, so the next emit paints every subtree. |

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;
use std::time::Duration;

fn app_tree() -> impl IntoNode {
    text("Hello")
}

fn main() -> sigil::AppResult<()> {
    let config: sigil::AppConfig = sigil::AppConfig::new("Demo").size(640, 400).terminal();
    let _ = (sigil::request_exit, sigil::app::is_grid, sigil::app::viewport_size);
    let _: sigil::AppResult<()> = Err(sigil::Error::with_kind(sigil::ErrorKind::Io, std::fmt::Error));
    let _ = sigil::app::PanicPolicy::Isolate;
    let _ = config.clone().frame_interval(Duration::from_millis(33));
    if let Ok(theme) = "base = \"dark\"\n[tokens]\naccent = \"#3c59e7\"\n".parse::<Theme>() {
        sigil::app::set_theme(theme);
    }
    let _: fn(std::path::PathBuf) -> sigil::AppResult<sigil::app::ThemeWatcher> = sigil::app::watch_theme;
#   #[cfg(feature = "desktop")]
    let _ = sigil::app::open("https://example.com");
    let mut host = sigil::Host::new(sigil::prelude::HostMode::Pixel);
    host.mount(app_tree);
#   #[cfg(feature = "shell")]
#   {
    let _ = sigil::app::WindowCommand::DragResize(sigil::app::ResizeDirection::East);
    let _ = sigil::app::Anchor::BottomCenter { margin: 16.0 };
#   }
    let _ = sigil::style::Token::Accent;
    let _ = sigil::scene::CommandScene::default();
#   #[cfg(feature = "scene")]

    let _ = sigil::terminal::focused();
    let _ = (sigil::prelude::cols(2.0), sigil::reactive::batch(|| 1));
    let _ = sigil::animation::Easing::Linear;
    let _ = sigil::animation::reduced_motion();
    let _ = sigil::ErrorKind::Closed;
    let _ = sigil::widgets::spacer();
    let _ = sigil::events::FocusIntent::Down;
    let _ = sigil::events::KeyPhase::Down;
    let _ = sigil::prelude::container();
    let _ = sigil::clipboard::paste();
    let _ = sigil::app::set_theme;
    sigil::run(config, app_tree)
}
# }
# fn main() {}
```

The modules `bidi`, `icons`, `profile`, `renderer`, `shell`, and `text` have no stable entry point in 0.1. Use them only through the functions above.

Feature-gated root items:

```rust,no_run
#[cfg(feature = "audio")]
fn play(path: &std::path::Path) -> sigil::AppResult<()> {
    sigil::app::Player::new().play_path(path)
}

#[cfg(feature = "pty")]
fn shell() -> sigil::AppResult<sigil::app::Session> {
    sigil::app::Session::spawn("sh")
}

#[cfg(feature = "dialog")]
fn pick() -> sigil::app::dialog::Picked<std::path::PathBuf> {
    sigil::app::dialog::FileDialog::new().filter("CSV", &["csv"]).open_file()
}

#[cfg(feature = "menu")]
fn menu_events() {
    drop(sigil::app::menu::Menu::events().subscribe(|id: String| println!("{id}")));
}

#[cfg(feature = "tray")]
fn tray_events() {
    drop(sigil::app::tray::Tray::events().subscribe(|event| println!("{event:?}")));
}

#[cfg(feature = "kittest")]
use sigil::testing as _;

fn main() {}
```

## Prelude

`use sigil::prelude::*;` imports every item in this section.

### Application

| Name | Signature | Purpose |
| --- | --- | --- |
| `Host::mount_hot` | `fn mount_hot<N: IntoNode>(&mut self, root: impl Fn() -> N + 'static)` (feature `hot`, not wasm) | Mount `root` through the `subsecond` jump table and connect to the `dx` devserver once. After each patch the next `Host::frame` calls `root` again, keeps the theme, and restores focus to the node with the same role and name path. `sigil::run` does this for its root. |
| `run` | `fn run(config, app) -> AppResult` | Same as `sigil::run`. |
| `AppConfig` | `struct AppConfig` | Same as `sigil::AppConfig`. |
| `AppResult` | `type AppResult<T>` | Same as `sigil::AppResult`. |
| `set_theme` | `fn set_theme(theme: Theme)` | Same as `sigil::app::set_theme`. |
| `watch_theme` | `fn watch_theme(path) -> AppResult<ThemeWatcher>` | Same as `sigil::app::watch_theme`. |
| `ThemeWatcher` | `struct ThemeWatcher` | Same as `sigil::app::ThemeWatcher`. |
| `open` | `fn open(target) -> AppResult<()>` | Same as `sigil::app::open` (feature `desktop`). |

```rust,no_run
# #[cfg(any(feature = "desktop", feature = "terminal"))]
# mod hosted {
use sigil::Error;
use sigil::app::is_grid;
use std::str::FromStr;
use sigil::app::viewport_size;
use sigil::prelude::*;
use sigil::Host;
use sigil::reactive::Owner;

fn tree() -> impl IntoNode {
    let size = viewport_size();
    text(derived(move || format!("{:?} grid={}", size.get(), is_grid())))
}

fn main() -> AppResult<()> {
    let _: Option<(Host, ThemeWatcher, Error)> = None;
    let _ = Owner::scope("demo", || text("x"));
    set_theme(Theme::from_str("base = \"light\"\n[tokens]\n").unwrap_or_else(|_| Theme::light()));
#   #[cfg(feature = "desktop")]
    let _ = open("https://example.com");
    let _ = watch_theme("theme.toml").map(drop);
    run(AppConfig::new("Demo"), tree)
}

#[cfg(feature = "audio")]
fn player() -> sigil::app::Player { sigil::app::Player::new() }
#[cfg(feature = "pty")]
fn session() -> AppResult<sigil::app::Session> { sigil::app::Session::spawn("sh") }
# }
# fn main() {}
```

### Nodes and text

| Name | Signature | Purpose |
| --- | --- | --- |
| `Cx` | `struct Cx`; `cx.add(node) -> NodeId` | The node tree. App code does not name it; a component is a function that returns `impl IntoNode`. Embedders and `on_added` hooks use it. |
| `NodeId` | `struct NodeId` | Handle of a node in the tree. |
| `NodeBuilder` | `struct NodeBuilder`; `child(impl IntoNode)`, `children_reactive(Fn() -> IntoIterator<Item = N>)`, `children_keyed(Fn() -> IntoIterator<Item = (K, N)>)`, `with<T>(value)`, `mutate(FnOnce(&mut Node))`, `add(&mut Cx) -> NodeId`; accessibility: `role(Role)`, `label(name)`, `description(text)`, `placeholder(text)`, `labelled_by(NodeId)`, `owns(NodeId)`, `action(Action)`, `keyboard_shortcut(shortcut)`, `focusable(bool)`, `tab_index(i32)`, `traps_tab(bool)`, `hittable(bool)`, `checked(Signal<bool>)`, `mixed(Signal<bool>)`, `selected(Signal<bool>)`, `expanded(Signal<bool>)`, `value(Signal<f32>, min, max)`, `value_step(f32)`, `text_value(Signal<String>)`, `text_cursor(caret, anchor)`, `row_count(Signal<usize>)`, `row_index(usize)`, `column_count(usize)`, `column_index(usize)`, `sort_direction(Signal<Option<SortDirection>>)`, `on_added`; `From` every widget builder | Builder that `container` and most widgets return. `NodeBuilder::from(text(..))` or `.into()` puts any widget builder into a `Vec<NodeBuilder>`; the result is the widget's own node, so `.layout()`, `.class()`, and accessibility settings called on it apply to the widget. `children_reactive` builds the children again each time a signal that the closure reads changes. `children_keyed` reconciles the children by key, like `for_each`. `with` attaches a typed value to the node. `hittable(false)` lets pointer events pass through the node. The accessibility methods set what screen readers get for the node. |
| `IntoNode` | `trait IntoNode` | A value that builds into a node. |
| `node` | `fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode>` | Box a child of any builder type. |
| `container` | `fn container() -> NodeBuilder` | A box that holds children. |
| `fragment` | `fn fragment() -> NodeBuilder` | Children without a box of their own. |
| `show` | `fn show<A: IntoNode, B: IntoNode>(cond: impl Reactive<bool>, then: impl Fn() -> A, otherwise: impl Fn() -> B) -> NodeBuilder` | Mounts `then()` while `cond` is true and `otherwise()` while it is false. `cond` is a `Signal<bool>` or a closure. The mounted branch keeps its node until `cond` flips. |
| `for_each` | `fn for_each<T, K: Clone + Eq + Hash, N: IntoNode>(items: Signal<Vec<T>>, key: impl Fn(&T) -> K, render: impl Fn(&T) -> N) -> NodeBuilder` | One child per item, reconciled by key. A row whose key stays keeps its node and state; new keys mount, missing keys unmount, and the order follows `items`. `render` runs for every item on each change, and only the rows of new keys mount. A row reads signals to update in place. A row helper `fn row(item: &T) -> impl IntoNode` captures the borrow under edition 2024; return a concrete builder such as `NodeBuilder`, or write `-> impl IntoNode + use<>`. |
| `switch` | `fn switch<K: Clone + PartialEq>(key: impl Reactive<K>)`; `.case(key: K, view: impl Fn() -> N)`, `.otherwise(view: impl Fn() -> N)`, `.keep_alive(bool)` | Mounts the view of the case whose key equals `key`, or `otherwise` when no case matches. Without `keep_alive`, only the active case is mounted and the others unmount. With `keep_alive(true)`, a case mounts the first time it is active, and after that it is hidden while inactive, so it keeps its state. `show` is the two-way form. |
| `text` | `fn text(source: impl Into<TextContent>) -> TextBuilder` | The one text node. `source` is a `&str` or `String` (static), a `Signal<T: Display>` (live), or a closure `Fn() -> String` (recomputed when the signals it reads change). `.nowrap()` keeps it on one line. |
| `TextContent` | `enum TextContent`; `From<&str>`, `From<String>`, `From<Signal<T: Display>>`, `From<F: Fn() -> String>` | Static, signal, or computed text. Every text-taking builder accepts it. |
| `image` | `fn image(source: ImageSource) -> ImageBuilder` | An image node from an `ImageSource`. |
| `ImageSource` | `ImageSource::from_rgba8(w, h, rgba) -> Option<ImageSource>`; `ImageSource::decode(encoded: &[u8]) -> Option<ImageSource>` (feature `image-decode`) | Decoded RGBA pixels. `decode` reads PNG, JPEG, and other formats and caches by content. |
| `Role` | `#[non_exhaustive] enum Role`; `Role::from_accesskit(accesskit::Role)` | Accessibility role. |
| `Action` | `#[non_exhaustive] enum Action { Click, Focus, Blur, Collapse, Expand, Decrement, Increment, ScrollIntoView, SetTextSelection, SetValue, ShowContextMenu }` | Accessibility action. |

```rust,no_run
use sigil::prelude::*;
use sigil::{Node, Surface, TopLayer};

fn tree() -> impl IntoNode {
    let pixels = ImageSource::from_rgba8(1, 1, vec![255, 0, 0, 255]);
    let picture = pixels.map(image).map(|i| i.fit(ObjectFit::Contain));
    let heading: TextBuilder = text("Title").role(Role::Heading);
    let body = container().label("Body").action(Action::Click);
    let marked = container().mutate(|n: &mut Node| drop(n.text_cursor()));
    let items = signal(vec![1_u32, 2, 3]);
    let open = signal(true);
    let rows = for_each(items, |n: &u32| *n, |n: &u32| text(n.to_string()));
    let panel = show(open, || text("Open"), || text("Closed"));
    let children: Vec<Box<dyn IntoNode>> = vec![node(heading), node(text(|| "live".into())), node(rows), node(panel)];
    let _: Option<(Surface, TopLayer)> = None;
    let mut root: NodeBuilder = fragment().child(body).child(marked);
    for child in children { root = root.child(child); }
    if let Some(picture) = picture { root = root.child(picture); }
    container().child(root)
}

fn main() {
    let _ = tree;
}
```

### Layout

| Name | Signature | Purpose |
| --- | --- | --- |
| `row` | `fn row() -> NodeBuilder` | A container that places its children left to right, centred across the row, with the theme gap: 8 px in a window, one cell in a terminal. Set `align_items` to stretch them. |
| `column` | `fn column() -> NodeBuilder` | A container that places its children top to bottom with the theme gap. |
| `LayoutStyle` | `LayoutStyle::flex_column()`, `flex_row()`, `grid(cols)`, `block()`; `LayoutStyle::pane(w, h)`; size: `width(d)`, `height(d)`, `fixed_size(w, h)`, `fill()`, `fill_width()`, `fill_height()`, `min_width(d)`, `min_height(d)`, `min_size(w, h)`, `max_width(d)`, `max_height(d)`, `max_size(w, h)`; flex: `flex_wrap(FlexWrap)`, `flex_basis(d)`, `grow(f32)`, `shrink(f32)`; align: `align_items(AlignItems)`, `align_self(AlignItems)`, `justify_content(JustifyContent)`; grid: `grid_columns(Vec<TrackSize>)`, `grid_rows(Vec<TrackSize>)`, `grid_area(col, row)`, `grid_column_span(n)`, `grid_row_span(n)`, `grid_dense()`; position: `display(Display)`, `position(Position)`, `inset(v)`, `inset_top/right/bottom/left(v)`; `overflow(Overflow)`, `overflow_x(Overflow)`, `overflow_y(Overflow)` | Layout description of a node. `fill_width`, `fill_height`, and `fill` take the full size of the parent on that axis. `grow` also sets an `auto` minimum size to 0, so the node can shrink below its content. `pane(w, h)` is `flex_column().fixed_size(w, h).gap(4.0)`. `grid_area` sets the start column and row. |
| `LayoutStyle` spacing | `padding(v)`, `padding_x/y(v)`, `padding_top/right/bottom/left(v)`, `margin(v)`, `margin_x(v)`, `margin_y(v)`, `margin_top(v)`, `margin_right(v)`, `margin_bottom(v)`, `margin_left(v)`, `gap(v)`, `gap_x(v)`, `gap_y(v)` | Padding, margin, and gap in px or text units. `margin_*` take the same type as `margin`; `gap_x` and `gap_y` take the same type as `gap`. |
| `Widget` | `fn map_node(FnOnce(NodeBuilder) -> NodeBuilder)`, `layout(LayoutStyle)`, `width`, `height`, `grow`, `asymmetric`, plus the style, state, event, drag, and reactive-position methods below | The one setter trait. `NodeBuilder`, `TextBuilder`, `ImageBuilder`, and every widget builder implement it. `layout` merges only the fields its `LayoutStyle` sets; a widget keeps the rest. `layout`, `width`, `height`, and `grow` apply in call order on every builder, and `width` also lowers a larger pixel minimum width. On a widget builder, `map_node` queues a change to the root `NodeBuilder` of the widget, for example `.map_node(\|n\| n.on_added(f))`, and the widget applies queued changes once, after its own build. Sealed. |
| `Widget` reactive position | `fn position_reactive`, `size_reactive`, `bounds_reactive` | Write the laid-out position or size to a signal. |
| `Display` | `enum Display { Block, Flex, Grid, None }` | Display mode. |
| `FlexWrap` | `enum FlexWrap { NoWrap, Wrap, WrapReverse }` | Wrap mode of a flex box. |
| `AlignItems` | `enum AlignItems { Start, End, Center, Baseline, Stretch }` | Cross-axis alignment of children. |
| `JustifyContent` | `enum JustifyContent` | Main-axis distribution of children. |
| `rows` | `const fn rows(count: f32) -> Dimension` | Height of `count` text lines. |
| `cols` | `const fn cols(count: f32) -> Dimension` | Width of `count` character cells. |
| `em` | `const fn em(count: f32) -> Dimension` | `count` times the font size. |

```rust,no_run

use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let size = signal((0.0_f32, 0.0_f32));
    let grid = LayoutStyle::grid(vec![TrackSize::Px(120.0), TrackSize::Fr(1.0), TrackSize::Auto]);
    let column = LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)).min_width(em(20.0));
    let row = LayoutStyle::flex_row().flex_wrap(FlexWrap::Wrap).align_items(AlignItems::Center);
    let row = row.justify_content(JustifyContent::SpaceBetween).align_self(AlignItems::Stretch);
    let abs = LayoutStyle::block().position(Position::Absolute).inset(Dimension::Px(0.0));
    let abs = abs.overflow(Overflow::Hidden).display(Display::Flex);
    let _ = (rows(2.0), cols(1.0), em(1.0));
    let button = button("OK").width(80.0).grow(1.0);
    container()
        .layout(column)
        .child(container().layout(grid).size_reactive(size))
        .child(container().layout(row).child(button))
        .child(container().layout(abs))
}

fn main() {
    let _ = tree;
    fn _ext<T: Widget>(_: T) {}
}
```

A row or column is as wide as its content. When a child fills, with a `Fill` width or a `grow` factor in a row, the container takes the width that its parent offers, and a row shares that width among its children. A `text_field` fills, so a row with a field and a button keeps the button on screen.

See [Units](guide/units.md) for the rules that map `rows`, `cols`, and `em` to pixels and cells.

### State

| Name | Signature | Purpose |
| --- | --- | --- |
| `Signal` | `struct Signal<T>`; `get`, `peek`, `try_peek`, `set`, `update`, `with`; `PartialEq`, `Eq`, `Hash` by identity | Reactive value. `set` skips a value equal to the current one; `update` always notifies. Two handles are equal when they name the same signal, so a store of signals can itself live in a signal. `try_peek` returns `None` once the signal's owner is disposed. |
| `signal` | `fn signal<T: 'static>(initial: T) -> Signal<T>` | Create a signal. |
| `derived` | `fn derived<T: Clone + PartialEq, F: FnMut() -> T>(f: F) -> Signal<T>` | A signal that `f` recomputes. Notifies readers only when the new value differs from the old one. |
| `effect` | `fn effect<F: FnMut() + 'static>(f: F)` | Run `f` now and after each change it reads. |
| `Reactive` | `trait Reactive<T>`; `fn read(&self) -> T`; `fn into_signal(self) -> Signal<T>` | A value source: a `Signal<T>`, a closure `Fn() -> T`, or a plain value (`bool`, `String`, `&'static str` as `Reactive<String>`, `Vec<T>` or `[T; N]` as `Reactive<Vec<T>>`). Show/hide, `invalid_when`, and the content widgets `diff`, `mermaid`, and `sparkline` take one. `disabled` on every widget takes one, so `.disabled(move \|\| name.get().is_empty())` needs no `derived`. |
| `interval` | `fn interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId` | Run `callback` on a fixed period. |
| `IntervalId` | `struct IntervalId`; `cancel()` | Stops the interval or timeout. |

```rust,no_run
use sigil::reactive::batch;
use sigil::reactive::defer;
use sigil::reactive::on_cleanup;
use sigil::reactive::untrack;
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let count: Signal<i32> = signal(0);
    let double = derived(move || count.get() * 2);
    let even = derived(move || count.get() % 2 == 0);
    effect(move || println!("{} {}", double.get(), even.get()));
    on_cleanup(|| println!("disposed"));
    batch(|| { count.set(1); count.update(|n| *n += 1); });
    effect(move || untrack(|| count.update(|n| *n += 1)));
    let inbox: Signal<Option<String>> = signal(None);
    effect(move || {
        if let Some(line) = inbox.get() {
            println!("{line}");
            defer(move || inbox.set(None));
        }
    });
    let len = count.with(|n| n.to_string().len());
    let source: Box<dyn Reactive<i32>> = Box::new(move || count.get() + len as i32);
    let id: IntervalId = interval(Duration::from_secs(1), move || count.update(|n| *n += 1));
    id.cancel();
    println!("{}", source.read());
    text(derived(move || count.get().to_string()))
}

fn main() {
    let _ = tree;
}
```

### Tasks

| Name | Signature | Purpose |
| --- | --- | --- |
| `task` | `fn task<T: Send, E: Send, F: Fn() -> Fut, Fut: Future<Output = Result<T, E>> + Send>(make: F) -> Task<T, E>` | Async work that runs on demand. Each run calls `make` for a new future. |
| `Task` | `struct Task<T, E>` (Copy); `run()`, `cancel()`, `status() -> Status<T, E>` | `run` sets `Pending`, aborts the previous run, and starts a new one; a result from a replaced run is dropped. `cancel` aborts the run in flight and sets `Pending` back to `Idle`; a finished status stays, and a result that arrives after `cancel` is dropped. `status` reads and subscribes; it needs `T: Clone` and `E: Clone`. The owner's cleanup aborts a pending run. |
| `spawn` | `fn spawn<T, E, F: Future<Output = Result<T, E>> + Send>(future: F) -> Signal<Status<T, E>>` | Start a future now, off the UI thread, and write its result to the signal. Runs once and cannot be cancelled; the owner's cleanup aborts it. For work that reruns or cancels, use `task`. For a future that owns state it cannot rebuild, such as a channel receiver. |
| `tasks::sleep` | `async fn sleep(duration: Duration)` | Wait inside a spawned future without blocking a thread. Uses the tokio timer on native and `setTimeout` on the web. |
| `tasks::blocking` | `async fn blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(f: F) -> T` | Run blocking work, such as a subprocess or a file read, inside a `task` or `spawn` future without stalling a runtime worker. Native: runs `f` on the tokio blocking pool. Web: runs `f` inline. A panic in `f` makes the task `Panicked`. Cancelling the task drops the result; `f` still runs to the end. |
| `Status` | `enum Status<T, E> { Idle, Pending, Ok(T), Err(E), Panicked(String) }`; `is_pending()`, `is_ok()`, `is_err()`, `is_panicked()` | State of a task. A `Task` is `Idle` until its first run; `spawn` starts `Pending`. |
| `signal_writer` | `fn signal_writer<T: Send>(signal: Signal<T>) -> SignalWriter<T>` | A `Send` handle that writes a signal. Panics outside a reactive scope, or when no runtime is installed with `AsyncRuntime::install_current`. |
| `SignalWriter` | `set(value) -> Result<(), Error>`, `update(f) -> Result<(), Error>`; `Clone` | Write a signal from another thread. Clone one writer into each `task` run to stream tokens into a signal; no channel is needed. |
| `Error` | `sigil::Error`; `kind() -> ErrorKind` | The crate error. A cross-thread write that was not queued has kind `ErrorKind::Closed`. |

```rust,no_run
use sigil::tasks::on_message;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let log = signal(String::new());
    let writer: SignalWriter<String> = signal_writer(log);
    std::thread::spawn(move || {
        let _: Result<(), Error> = writer.update(|s| s.push_str("line\n"));
    });
    let fetched: Signal<Status<u32, String>> = spawn(async { Ok::<u32, String>(42) });
    let reload: Task<u32, String> = task(|| async { Ok(7) });
    reload.run();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    on_message(rx, move |line| log.update(|s| s.push_str(&line)), || {});
    let (done_tx, done_rx) = tokio::sync::mpsc::channel::<String>(16);
    on_message(done_rx, move |line| log.set(line), || println!("worker closed"));
    drop(done_tx);
    drop(tx.send("hello\n".into()));
    text(derived(move || match (fetched.get(), reload.status()) {
        (Status::Ok(n), Status::Ok(m)) => format!("{n} {m}"),
        _ => log.get(),
    }))
}

fn main() {
    let _ = tree;
}
```

### Events and input

| Name | Signature | Purpose |
| --- | --- | --- |
| `Widget` events | `key`, `key_any`, `key_capture`, `key_capture_any`, `shortcut`, `pointer_down`, `on_scroll`, `on_gesture`, `on_paste`, `focus_in`, `link`, `on_selection_copy`, `autofocus`, `focus_when(Signal<u64>)`, ... | Attach event handlers to any builder. On a widget builder the handler goes on the root node of the widget, after the handlers of the widget. `autofocus()` gives the node keyboard focus when it mounts, and again each time a focus scope that holds it, such as a modal, opens; `focus_when(trigger)` focuses it each time the signal changes, with no `NodeId` plumbing. `link(f)` receives the URL when a link inside the node, such as one in `markdown`, is clicked; the deepest handler on the path gets it. `on_selection_copy(f)` receives the text and the clipboard result each time a `selectable_text` region inside the node copies a selection. |
| `request_focus` | `fn request_focus(id: NodeId)` | Move keyboard focus to a node. |
| `Shortcut` | `Shortcut::new(key, mods)`, `plain(key)`, `ctrl(key)`, `primary(key)`, `label()`, `label_for(mac: bool)`; `FromStr` (`"ctrl+shift+k".parse::<Shortcut>()`) | A key plus modifiers. With Shift a character key matches in either case. `parse` reads the names `label()` prints, in either form (`⇧⌘K` or `Shift+Super+K`), plus `cmd`, `ctrl`, `alt`, `option`; an unknown name is an `Error` with kind `ErrorKind::UnknownModifier` or `ErrorKind::UnknownKey`. `label()` prints it for the running platform: `⇧⌘K` on macOS, `Shift+Super+K` elsewhere; `label_for` picks the style explicitly. `primary` uses `events::PRIMARY`: Command on macOS, Control elsewhere, for example `Shortcut::primary(LogicalKey::character("s"))` for Cmd+S or Ctrl+S. |
| `Key` | `#[non_exhaustive] enum Key` | A named key such as `Key::Enter`. |
| `LogicalKey` | `LogicalKey::named(key)`, `LogicalKey::character(s)` | A named key or a character. |
| `ModifiersState` | `struct ModifiersState`; `SHIFT`, `CONTROL`, `ALT`, `META`, `empty`, `contains`, `intersects`, `union`, `intersection`, `difference`, `set`, `insert`, `remove`, `shift`, `ctrl`, `alt`, `meta`, and `\|` | Modifier flags such as `CONTROL`. |
| `KeyEvent` | `struct KeyEvent { key, physical, modifiers, repeat, phase, text }` | A key press or release. |
| `KeyPhase` | `enum KeyPhase { Down, Up }` | Press or release. |
| `PointerEvent` | `struct PointerEvent` | A pointer event. `capture_pointer()` sends the following moves and the release to the node that asked, and only to it: they do not bubble to its ancestors. `position` is relative to the handler's node; `window_position` is in window coordinates. `pointer_id` tells touch contacts apart (0 for the mouse); `pressure` and `tilt` are `Some` for touch and pen input that reports them. |
| `PointerKind` | `enum PointerKind { Mouse, Touch, Pen }` | Pointer device. |
| `PointerPhase` | `enum PointerPhase { Down, Move, Up, Enter, Leave, Cancel }` | Pointer event phase. |
| `MouseButton` | `enum MouseButton { Left, Right, Middle, Back, Forward, Other(u16) }` | One mouse button. |
| `MouseButtons` | `struct MouseButtons`; `MouseButtons::LEFT` | Set of pressed buttons. |
| `Point` | `struct Point { x: f32, y: f32 }` | A position in logical px. |
| `Size` | `Size::fixed(w, h)`, `Size::max_content()`, `Size::min_content()`, `with_fixed_width(w)`, `fixed_width() -> Option<f32>` | A size constraint. Construct it with the methods. `fixed_width` is the fixed width, or `None` for min-content or max-content. |

```rust,no_run
# #[cfg(feature = "shell")]
use sigil::app::ResizeDirection;
# #[cfg(feature = "shell")]
use sigil::app::WindowCommand;
use sigil::events::FileDropPhase;
use sigil::events::FocusIntent;
use sigil::events::ImeEvent;
use sigil::events::PhysicalKey;
use sigil::events::ScrollDelta;
use sigil::events::ScrollEvent;
use sigil::events::ScrollPhase;
use sigil::events::WindowEvent;
# #[cfg(feature = "desktop")]
use sigil::events::request_window_command;
use sigil::geometry::Size;
use sigil::geometry::LogicalSize;
use sigil::prelude::*;
use sigil::events::Event;

fn tree() -> impl IntoNode {
    let save = Shortcut::ctrl(LogicalKey::character("s"));
    let quit = Shortcut::new(LogicalKey::character("q"), ModifiersState::empty());
    let root = container()
        .focusable(true)
        .key(save, |event: &mut KeyEvent| event.stop_propagation())
        .key(Shortcut::plain(Key::Escape), |_| sigil::request_exit())
        .key_capture(quit, |_| sigil::request_exit())
        .shortcut(Shortcut::ctrl(LogicalKey::character("o")), |_| drop(()))
        .key_any(|event| drop(event.phase == KeyPhase::Down && event.text.is_some()))
        .key_capture_any(|event| drop(matches!(event.key, LogicalKey::Named(Key::Enter))))
        .pointer_down(|event: &mut PointerEvent| drop(event))
        .on_scroll(|event: &mut ScrollEvent| drop(event))
        .file_drop(|event| drop(event))
        .on_ime(|event: &ImeEvent| drop(matches!(event, ImeEvent::Commit(_))))
        .on_added(|_, id| request_focus(id));
#   #[cfg(feature = "desktop")]
    request_window_command(WindowCommand::DragResize(ResizeDirection::South));
    root
}

fn main() {
    let _ = tree;
    let _ = (PointerKind::Mouse, PointerPhase::Down, MouseButton::Left, MouseButtons::LEFT);
    let _ = (ScrollDelta::Lines(Point { x: 0.0, y: 1.0 }), ScrollPhase::Moved, FileDropPhase::Dropped);
    let _ = (FocusIntent::Down, PhysicalKey::KeyA, Size::min_content().with_fixed_width(1.0));
    let _ = LogicalSize { width: 80.0, height: 24.0 };
    let _: Option<(Event, WindowEvent)> = None;
}
```

### Style

| Name | Signature | Purpose |
| --- | --- | --- |
| `Widget` style | `theme`, `tone_scope`, `focus_ring(Signal<bool>)`, `focus_ring_color(impl Into<ColorAtom>)`, `focus_ring_width(f32)`, `focus_ring_offset(f32)`, `focus_ring_opacity(u16)`, `presence(Signal<bool>, ..)`, `animate_opacity`, `animate_height`, and one `animate_*` setter per numeric property, `visible_when(impl Reactive<bool>)`, `paint_when(impl Reactive<bool>, \|s\| ..)` | Theme, tone, focus ring, presence, and animation of any builder. `visible_when` keeps the node mounted and hides it while the source is false; `show` mounts and unmounts instead. `paint_when` applies the paint from its closure while the source is true. `presence` animates the node in and out. |
| `Paint` | `bg(c)`, `text_color(c)`, `border_color(c)`, `border_w(px)`, `border_top/right/bottom/left(px)`, `rounded(Radius)`, `layer(Layer)`, `shadow(Elevation)`, `opacity(f32)`, `text_role(TextRole)`, `font_size(f32, reason)`, `font_weight(u16)`, `font_italic(bool)`, `font_family(FontFamily)`, `line_height_px(px)`, `letter_spacing_em(em)`, `tabular_nums(bool)`, `underline()`, `line_through()` | Typed paint on nodes, text, widget builders, and state closures. `c` is `impl Into<ColorAtom>`. Sealed. |
| `Paint` backgrounds | `bg_linear(GradientAxis)`, `bg_radial(Radial)`, `bg_conic(impl Into<Conic>)`, `gradient_from(c)`, `gradient_via(c)`, `gradient_to(c)`, `gradient_from_position(u8)`, `gradient_via_position(u8)`, `gradient_to_position(u8)`, `bg_clip_text()`, `bg_image(ImageSource)` | Tailwind `bg-linear-to-*`, `bg-radial`, `bg-conic-*`, `from-*`, `via-*`, `to-*`, `from-<n>%`, `via-<n>%`, `to-<n>%`, and `bg-clip-text`. Stops default to Tailwind's: `from` and `to` are transparent, at 0%, 50%, and 100%; `via` is drawn only when set. Stops mix in premultiplied gamma-encoded sRGB, as CSS does for legacy colors, on every renderer; a stop placed before an earlier stop moves up to the earlier position (CSS Images 4 §3.5.3). `bg_clip_text` paints the gradient, in the node's box, through the glyphs of the node and its descendant text. `bg` paints under the gradient. `bg_image` tiles the image at its natural size inside the border box; the node owns the image. Fills, strokes, and gradient text follow the node's rounded corners. |
| `Paint` borders | `border_solid()`, `border_dashed()`, `border_dotted()` | Tailwind `border-solid`, `border-dashed`, and `border-dotted`. Dashes follow corner arcs. A dash is 3 widths with a 2-width gap below a 3 px width, and 2 widths with a 1-width gap from 3 px, as Chromium draws them. Dots wider than 3 px are round, one every 2 widths; thinner dots are square. The period is fitted so a whole number of dashes fills the perimeter. |
| `Paint::clip_path` | `fn clip_path(self, shape: ClipShape) -> Self` | Clips the node, its own background, and its children to `shape`, like CSS `clip-path`. The shape moves with the node's transform. A pointer outside the shape misses the node. |
| `Paint` masks | `mask_image(MaskImage)`, `mask_stops(from, to)`, `mask_luminance()`, `mask_size(MaskSize)`, `mask_position(x, y)`, `mask_repeat(bool)`; `enum MaskImage { Linear(degrees), Radial(Radial), Conic(Conic), Image(ImageSource) }`, `enum MaskSize { Auto, Cover, Contain }` | CSS `mask-image`, `mask-mode`, `mask-size`, `mask-position` and `mask-repeat` on the node and its children, measured against the border box and clipped to it. Gradient masks go from opaque at `from` (default 0%) to clear at `to` (default 100%) and fill the box. Image masks draw at their natural size unless sized, at `mask_position` fractions of the free space (default 0, 0), and tile unless `mask_repeat(false)`. Classes: `mask-linear-<deg>`, `mask-linear-from-<n>%`, `mask-linear-to-<n>%`, `mask-{t,r,b,l}-from-<n>%` and `-to-<n>%`, `mask-radial-from-<n>%`, `mask-radial-to-<n>%`, `mask-circle`, `mask-ellipse`, `mask-radial-closest-side` (and the other extents), `mask-radial-at-<place>`, `mask-conic-<deg>`, `mask-conic-from-<n>%`, `mask-conic-to-<n>%`, `mask-none`, `mask-alpha`, `mask-luminance`, `mask-auto`, `mask-cover`, `mask-contain`, `mask-repeat`, `mask-no-repeat`, and `mask-<place>` such as `mask-center` or `mask-top-left`. One mask image per node: two edge classes do not intersect, the last one wins. Stops take percentages only, not spacing numbers or colors. |
| `Paint::mix_blend` | `fn mix_blend(self, mode: BlendMode) -> Self` | CSS `mix-blend-mode`: the node and its children draw as one layer mixed with what is behind it. Classes `mix-blend-normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`, `color-dodge`, `color-burn`, `hard-light`, `soft-light`, `difference`, `exclusion`, `hue`, `saturation`, `color` and `luminosity`; the last one wins. |
| `Paint` filters | `blur(px)`, `brightness(amount)`, `contrast(amount)`, `grayscale(amount)`, `hue_rotate(degrees)`, `invert(amount)`, `saturate(amount)`, `sepia(amount)`, `drop_shadow(impl Into<Option<Shadow>>)`, `text_shadow(impl Into<Option<Shadow>>)` | CSS `filter` and `text-shadow`. Amounts are fractions, 1.0 for 100%. Blur and shadow blur are standard deviations in px. The filters apply in Tailwind's order, blur, brightness, contrast, grayscale, hue-rotate, invert, saturate, sepia, drop-shadow, whatever order they were set in; each kind keeps its last value. They apply before `clip_path`, masks and opacity, inside the transform. `text_shadow` draws under the glyphs of a text node and its descendant text, as CSS `text-shadow` inherits; a descendant's own `text_shadow` replaces it, and `text_shadow(None)` or `text-shadow-none` cancels it for that node and its descendants. Classes: `blur-xs` to `blur-3xl` and `blur-none`, `brightness-<n>`, `contrast-<n>`, `saturate-<n>`, `grayscale`, `invert`, `sepia` and their `-<n>` forms, `hue-rotate-<n>` and `-hue-rotate-<n>`, `drop-shadow-xs` to `drop-shadow-2xl`, `drop-shadow-none`, `drop-shadow-<color>`, `text-shadow-2xs` to `text-shadow-lg`, `text-shadow-none` and `text-shadow-<color>`, with Tailwind 4.1 values; a text shadow's blur radius becomes a deviation of half the radius. |
| `Paint` 3D | `perspective(px)`, `perspective_origin(x, y)`, `rotate_x(deg)`, `rotate_y(deg)`, `rotate_z(deg)`, `translate_z(px)`, `backface_hidden()`, `backface_visible()` | Tailwind `perspective-*`, `perspective-origin-*`, `rotate-x-*`, `rotate-y-*`, `rotate-z-*`, `translate-z-*`, `backface-hidden`, and `backface-visible`. `perspective` and its origin apply to the node's children. The turns and depth follow the node's `transform` in CSS order, around its transform origin. Hit testing and accessibility bounds use the projected quad. Flat children only: there is no `transform-style: preserve-3d`. Keyframes animate them through `Property::RotateX`, `RotateY`, `RotateZ`, `TranslateZ`, and `Perspective`; `transition` and `transition_transform` cover the turns and depth. |
| `Widget` states | `fn rest(f)`, `hover(f)`, `focus(f)`, `pressed(f)`, `disabled_style(f)`; `f: FnOnce(StateStyleBuilder)` | Paint per interaction state. Works on a plain node, a text node, and a widget. |
| `Token` | `enum Token { Accent, Surface, OnSurface, Border, .. }` | A theme color. Pass it to `bg`, `text_color`, or `border_color`. |
| `Radius` | `enum Radius { None, Sm, Base, Md, Lg, Full }` | Corner radius from the theme: 2, 4, 6, 8, and 9999 px. 0 on a character grid. |
| `Elevation` | `enum Elevation { Sm, Base, Md, Lg, Xl }` | Shadow depth from the theme. |
| `Space` | `enum Space { None, Xxs, Xs, Sm, Md, Lg, Xl, Xxl, Xxxl }`, `Space::raw(px, reason)` | Steps of the spacing scale: 0, 2, 4, 8, 12, 16, 24, 32, and 48 px. `gap`, `padding`, and `margin` setters take a `Space` or a text length such as `rows(1.0)`, not a bare number. |
| `TextRole` | `enum TextRole { Caption, Body, Label, Subtitle, Title, Heading, Display }` | `.text_role(role)` sets size, line height, and weight together. `text_scale` scales them. |
| `Layer` | `enum Layer { Page, Sunken, Raised, Overlay }` | A place in the visual stack. `.layer(Layer::Raised)` sets the fill, border, corners, and shadow together. `card` is `Raised` and popups are `Overlay`. The terminal host draws no shadows. |
| `FlawKind` | `enum FlawKind { Column, Inset, ClippedText, UnevenControls, Overlap, Overflow, Contrast, SmallTarget }` | A design rule. Debug builds report each break once through `tracing` target `sigil::design`. `Harness` fails on one. |
| `Widget::allow_flaw` | `fn allow_flaw(self, kind: FlawKind, reason: &'static str) -> Self` | Opt a node and its children out of one design rule. |
| `ControlSize` | `enum ControlSize { Compact, Default, Wide, Grow }` | Size step of a control, as in `button(..).size(ControlSize::Compact)`. |
| `FontFamily` | `enum FontFamily { Sans, Mono, Serif, Icon, Display }` | Font family for `font_family`. |
| `Rgba8` | `struct Rgba8 { r, g, b, a: u8 }` | An 8-bit RGBA color. |
| `GradientAxis` | `enum GradientAxis { TopToBottom, BottomToTop, LeftToRight, RightToLeft }` | The direction of a linear background gradient. |
| `Radial` | `struct Radial`; `Radial::new()`, `.circle(bool)`, `.extent(RadialExtent)`, `.at(x: u8, y: u8)`, `.repeating(bool)`; `Default` | A CSS radial gradient shape: an ellipse by default, sized to `RadialExtent::FarthestCorner`, centred; `at` takes percents of the box. |
| `RadialExtent` | `enum RadialExtent { ClosestSide, ClosestCorner, FarthestSide, FarthestCorner }` | How far a radial gradient reaches, as in CSS. `FarthestCorner` is the default. |
| `Conic` | `struct Conic`; `Conic::new(from_degrees: i16)`, `.at(x: u8, y: u8)`, `.repeating(bool)`; `From<i16>` | A CSS conic gradient: it starts at `from_degrees`, clockwise from the top, around a point in percents of the box. |
| `ClipShape` | `enum ClipShape { Inset { inset: [i16; 4], round: i16 }, Circle(Option<i16>), Ellipse, Polygon(&'static [(f32, f32)]) }` | A CSS `clip-path` basic shape against the border box. Insets and radii are px; `Circle(None)` reaches the closest side; polygon points are fractions of the box. |
| `ColorAtom` | `enum ColorAtom { Rgba8(Rgba8), Token(Token), Custom(&'static str) }` | A literal color, a theme token, or a custom token name. `From<Rgba8>` and `From<Token>`. |
| `Token::scale` / `Token::scale_step` | `fn scale(ScaleRole, u8) -> Option<Token>`, `fn scale_step(self) -> Option<(ScaleRole, u8)>` | Map a scale role and step 1..=12 to its token and back. |
| `Theme` | `Theme::light()`, `Theme::dark()`, `Theme::console_dark()`; `with_direction(Direction)` | Theme values. The direction, locale, and strings apply to the whole tree: a right-to-left theme mirrors padding, margins, insets, flex row order, and start/end alignment; `number_field` and `date_field` parse and format with the locale; built-in labels such as Close and Cancel come from the strings. |
| `style::Direction` | `enum Direction { Ltr, Rtl }`; `is_rtl()`, `Direction::from_name("ltr" | "rtl")` | Layout direction. `.with(Direction::Ltr)` on a node keeps it and its children left to right in a right-to-left theme, as `code_edit` does. |
| `style::StringKey` | `struct Strings`; `Strings::english()`, `from_table(&[(StringKey, &str)])`, `with(key, value)`, `set`, `get(key)`, `fill(key, args)`; `enum StringKey`, `StringKey::from_name` (snake_case, for example `"close"`) | The built-in labels a theme carries, with English defaults. `fill` replaces `{0}`, `{1}` placeholders. |
| `Theme::resolve_color` | `fn resolve_color(&self, color: ColorAtom) -> Rgba8` | Resolve a color. Order: runtime override, theme table, built-in value. An unknown `Custom` name panics in debug builds and paints `missing_token_fallback` with one `tracing` warning per name in release builds. |
| `Theme::color` | `fn color(&self, token: Token) -> Rgba8` | `resolve_color` for a built-in token. |
| `Theme::register_token` / `with_token` | `fn register_token(&mut self, name: &str, value: ThemeValue) -> Option<ThemeValue>` | Set a runtime override. A built-in token name also updates the matching base color field. |
| `Tone` | `enum Tone { Neutral, Primary, Info, Success, Warn, Error }`; `token(step: u8) -> Token`, `shade()`, `fill()`, `solid()` | Semantic color family. `token` returns the tone's color-scale step (1–12); `shade` is step 4 (tinted background), `fill` step 9 (solid fill), `solid` step 11 (text and strokes). |

Paint calls write to one store per node. A later call wins for each property that it sets, whether it is a `Paint` method, a state variant, or `.layout`. A background, border, or size that you set on a widget holds in every state, over the widget's own defaults. To keep a widget's hover and press feedback in another color, set `.tone(..)` instead of a color. On a widget builder, the `Paint` methods win over `width`, `height`, `grow`, and `layout` for the same property.

```rust,no_run
use sigil::style::ColorAtom;
use sigil::style::ThemeValue;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let card = container()
        .layout(LayoutStyle::default().padding(Space::Sm).margin_x(Space::Lg).gap_y(Space::Xs))
        .bg(Rgba8 { r: 20, g: 20, b: 30, a: 255 })
        .rounded(Radius::Base)
        .shadow(Elevation::Md)
        .border_w(1)
        .border_color(Token::Border)
        .hover(|s| s.bg(Token::SurfaceHover))
        .disabled_style(|s| s.opacity(0.5))
        .child(text("Title").text_role(TextRole::Body).font_weight(600).text_color(Token::OnSurface))
        .child(text("42").font_family(FontFamily::Mono).tabular_nums(true));
    let _ = (ColorAtom::from(Token::Accent), Token::Accent.color(), ThemeValue::Ms(150));
    let theme = Theme::dark();
    let node = container().tone_scope(Tone::Primary).focus_ring(signal(true)).focus_ring_color(Token::Ring);
    fragment().child(node.theme(theme)).child(card)
}

fn main() {
    let _ = tree;
    fn _traits<T: Paint + Widget>(_: T) {}
}
```

### Animation

| Name | Signature | Purpose |
| --- | --- | --- |
| `Paint` transitions | `transition()`, `transition_all()`, `transition_colors()`, `transition_opacity()`, `transition_transform()`, `duration(Duration)`, `ease(Easing)`, `delay(Duration)` | Tailwind `transition`, `transition-all`, `transition-colors`, `transition-opacity`, `transition-transform`, `duration-*`, `ease-*`, and `delay-*`. A transition runs when a state closure or `paint_when` changes a covered property, and starts from the value on screen. The default is 150 ms on `Easing::IN_OUT`. `transition` covers colors, gradient stop colors, opacity, transform with the 3D turns and depth, and filters; the spring presets on widgets cover every animatable property. Filter lists interpolate per function when one list starts with the other's functions in order, and a missing function starts or ends at its initial value (blur 0, brightness 1, drop-shadow 0 px transparent), as in Filter Effects 1; any other pair of lists jumps. Gradient stop colors and positions interpolate when the gradient keeps its shape and stop count, and otherwise jump. A text shadow interpolates offset, blur, and color when both sides have at most one shadow. Colors mix in sRGB with premultiplied alpha, as CSS does for legacy colors. A zero `duration` with a `delay` holds the start value until the delay ends, then jumps. |
| `Paint::animate` | `fn animate(self, keyframes: Keyframes) -> Self` | Plays a keyframe animation while the setter is in effect, like Tailwind `animate-*`. It starts again when the setter comes back. With reduced motion the node shows its plain style. An animation set inside a reactive scope stays available while that scope lives. If an animation set outside any scope is dropped from the shared store before it plays, Sigil logs one `sigil::animation` warning and shows the plain style. |
| `Keyframes` | `Keyframes::new(Duration)`, `keyframe(offset, Property)`, `ease(Easing)`, `ease_at(offset, Easing)`, `iterations(u32)`, `infinite()`, `reverse()`, `alternate()`; `spin()`, `ping()`, `pulse()`, `bounce()` | A CSS `@keyframes` animation. Offsets run from 0.0 to 1.0 of one pass. A property with no keyframe at 0.0 or 1.0 takes the node's own value there. `ease` sets the curve between each pair of keyframes, CSS `ease` by default; `ease_at` sets it from one offset to the next. A finished animation shows the node's own style. `spin`, `ping`, `pulse`, and `bounce` are the Tailwind animations. |
| `Property` | `#[non_exhaustive] enum Property { Opacity(f32), TranslateX(px), TranslateY(px), TranslateXPercent(f32), TranslateYPercent(f32), Scale(f32), Rotate(radians), RotateX(radians), RotateY(radians), RotateZ(radians), TranslateZ(px), Perspective(px), Background(ColorAtom), TextColor(ColorAtom), BorderColor(ColorAtom), Blur(px), Brightness(f32), Contrast(f32), Grayscale(f32), HueRotate(degrees), Invert(f32), Saturate(f32), Sepia(f32), DropShadow(Shadow), TextShadow(Shadow), GradientFrom(ColorAtom), GradientVia(ColorAtom), GradientTo(ColorAtom), GradientFromPosition(percent), GradientViaPosition(percent), GradientToPosition(percent), MaskFrom(percent), MaskTo(percent) }` | A property value that a keyframe sets. Percent translation is of the node's own size, from -1600% to 1600% in steps of 1/2048 of the size. Colors mix in sRGB with premultiplied alpha. A filter function the node does not set starts from its initial value. `GradientVia` needs a gradient with a middle stop, `Perspective` a node that sets one, and the mask stops a gradient mask. Filter amounts resolve to 1%, hue to 1 degree, and stop and mask positions to 1%. |
| `Easing` curves | `Easing::cubic_bezier(x1, y1, x2, y2)`, `Easing::IN`, `Easing::OUT`, `Easing::IN_OUT`, `Easing::Linear` | CSS `cubic-bezier()` and the Tailwind `ease-in`, `ease-out`, `ease-in-out`, and `ease-linear` curves. A NaN or infinite control point gives `Easing::Linear`, and `apply(NaN)` returns the start of the curve. |

```rust,no_run
use std::time::Duration;

use sigil::animation::Easing;
use sigil::animation::{Keyframes, Property, clock, reduced_motion};
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let opacity = signal(0.0_f32);
    let open = signal(false);
    let _ = (Easing::CubicInOut, reduced_motion());
    let _seconds: Signal<f64> = clock();
    let fade = Keyframes::new(Duration::from_millis(400))
        .keyframe(0.0, Property::Opacity(0.0))
        .keyframe(1.0, Property::Opacity(1.0));
    column()
        .child(container().animate_opacity(opacity))
        .child(container().animate(Keyframes::spin()))
        .child(container().animate(fade))
        .child(
            container()
                .opacity(0.5)
                .transition()
                .duration(Duration::from_millis(300))
                .ease(Easing::OUT)
                .delay(Duration::from_millis(50))
                .paint_when(open, |state| state.opacity(1.0)),
        )
}

fn main() {
    let _ = tree;
}
```

### Widgets: controls and fields

| Name | Signature | Purpose |
| --- | --- | --- |
| `button` | `fn button(label: impl Into<TextContent>) -> ButtonBuilder` | A push button. |
| `icon_button` | `fn icon_button(glyph: impl Into<ButtonGlyph>, activate: impl Fn() + 'static) -> ButtonBuilder` | A button that shows one glyph: text or an `Icon`. An icon's name is the accessible name. |
| `checkbox` | `fn checkbox(checked: Signal<bool>)` | A checkbox. |
| `toggle` | `const fn toggle(on: Signal<bool>)` | An on/off switch. |
| `radio_group` | `fn radio_group(selected: Signal<String>)` | A set of radio options; `.label()` and `.labelled_by()` name the group. |
| `segmented` | `fn segmented(active: Signal<usize>, labels: Vec<String>)` | A framed row of options where one is active; `.label()` names the group and `.height()` (`Widget`) sets its height. |
| `slider` | `fn slider(value: Signal<f32>, range: RangeInclusive<f32>)`; `.label(text)` | A value slider. `.label` shows the text before the track and names the slider. |
| `Orientation` | `enum Orientation { Horizontal, Vertical }` | Orientation of a slider or splitter. |
| `text_field` | `fn text_field(value: Signal<String>) -> TextFieldBuilder`; `.leading(node)`, `.trailing(node)`, `.clearable()` | A single-line text input. `leading` and `trailing` place a node, such as an icon, inside the field border; the input fills the rest. `clearable` shows a clear button while the value is not empty; it empties the value, fires `change`, and puts focus back on the input. |
| `text_edit` | `fn text_edit(value: Signal<String>) -> TextEditBuilder`; `.autofocus()`, `.focus_when(Signal<u64>)`, `.caret_rect(Signal<Option<Rect>>)` | A multi-line text editor. `autofocus` focuses it when it mounts; `focus_when` focuses it each time the signal changes. `caret_rect` receives the caret rectangle, relative to the editor's box, each time the caret moves, and `None` while the editor is disabled, so a popup can follow the caret. |
| `number_field` | `fn number_field<T>(value: Signal<T>, range: RangeInclusive<T>)` for `i32`, `i64`, `usize`, `f32`, and `f64` | A numeric input. Text that parses updates `value` as you type; on Enter or blur, text that does not parse is replaced by the stored value, so the field never shows a number it did not store. |
| `date_field` | `fn date_field(value: Signal<Date>)` | A date input. |
| `Date` | `struct Date { year: i32, month: u8, day: u8 }` | A calendar date. `parse` and `render` use `Locale::default()`. |
| `color_field` | `fn color_field(value: Signal<Rgba8>)` | A color input. |
| `field` | `fn field<C: IntoNode>(label, control: C)`; `.error(impl Reactive<Option<String>>)` when `C: FieldControl`; `.required(impl Reactive<bool>)` | A label above a control. `.required` marks the control required for screen readers and shows a marker after the label while it is true. `.error` shows the message under the control and paints the control invalid while it is `Some`. The control is marked invalid while an error shows and is described by the message. |
| `chip` | `fn chip(label, tone: Tone)`; `.dot()`, `.leading(node)` | A small tag. `dot` puts a dot in the chip's tone before the label, for status pills; `leading` puts any node, such as an icon, before it. |
| `interactive` | `fn interactive(inner: NodeBuilder)` | Make any node hoverable, pressable, and focusable. |

```rust,no_run
use sigil::widgets::ButtonBuilder;
use sigil::widgets::TextEditBuilder;
use sigil::widgets::TextEditState;
use sigil::widgets::TextFieldBuilder;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let name = signal(String::new());
    let on = signal(false);
    let save: ButtonBuilder = button("Save").primary().activate(|| println!("saved"));
    let close: ButtonBuilder = icon_button("x", || sigil::request_exit());
    let _ = (checkbox(on), toggle(on));
    let _ = radio_group(signal("a".to_string()));
    let _ = slider(signal(0.5), 0.0..=1.0);
    let _ = Orientation::Vertical;
    let _: TextEditBuilder = text_edit(signal(String::new()));
    let _ = TextEditState::new(name, signal(0));
    let _ = number_field(signal(3_i32), 0..=10);
    let _ = date_field(signal(Date { year: 2026, month: 1, day: 1 }));
    let _ = (color_field(signal(Rgba8 { r: 0, g: 0, b: 0, a: 255 })), field("Name", text_field(name)));
    let checked = field("Name", text_field(name)).error(move || name.with(|n| n.is_empty().then(|| "Required".to_string())));
    let _ = chip("beta", Tone::Info);
    let _ = (chip("3", Tone::Error), text("Label").nowrap());
    let _ = interactive(container());
    let _: Option<TextFieldBuilder> = None;
    container().child(checked).child(save).child(close)
}

fn main() {
    let _ = tree;
}
```

### Widgets: choice, collections, and documents

| Name | Signature | Purpose |
| --- | --- | --- |
| `dropdown` | `fn dropdown(selected: Signal<usize>, options: Vec<String>)` | A select box. The trigger fills the width of its parent, like a form field; in a toolbar row give it `.width(..)`. |
| `combobox` | `fn combobox(state: Combobox) -> NodeBuilder` | A text input with a filtered list. |
| `Combobox` | `Combobox::new(options: Vec<String>)`; `selected_option() -> Option<String>`, `edit_query(text: &str)`, `move_cursor(delta: i32)` | State of a combobox. |
| `menu` | `fn menu(open: Signal<bool>)` | A popup menu. |
| `context_menu` | `fn context_menu(target, items: impl Fn() -> Vec<MenuItem>) -> NodeBuilder` | Wraps `target`; a right click opens a menu of `items()` at the pointer. |
| `MenuItem` | `struct MenuItem { label, shortcut, disabled, submenu_indicator }`; `MenuItem::new(label)`, `shortcut(hint)`, `disabled(bool)`, `submenu(bool)`, `activate(Fn())` | One menu entry. `activate` runs when the entry is pressed. |
| `menu_bar` | `fn menu_bar()`; `.fill()` | An in-window menu bar, as wide as its menus. `fill` makes it take the full width. |
| `command_menu` | `fn command_menu()` | A command palette. Typing in its query field filters and ranks items with `fuzzy_score`; Up and Down move, Enter or a click runs an item or opens a submenu, and Escape goes back one level. |
| `fuzzy_score` | `fn fuzzy_score(query: &str, text: &str)`, returns an `Option` of a value with `score: usize` and `positions: Vec<usize>` | Rank text against a query. A lower `score` is a better match. |
| `table` | `fn table<Row>(state: TableState<Row>)` | A sortable, filterable table. |
| `TableState` | `TableState::new(rows: Vec<Row>, columns: Vec<TableColumn<Row>>)`; `TableState::identified(rows, columns)` for `Row: Identifiable`; `replace_rows(impl Into<Arc<Vec<Row>>>)`, `row_eq(self, impl Fn(&Row, &Row) -> bool) -> Self`, `selected_row() -> Option<Row>`, `clear_selection()`, `show_column(id: &'static str)`, `reset_columns()`; sort: `sort(id, SortDirection)`, `toggle_sort(id)`, `cycle_sort()`, `reverse_sort()`, `sort_key() -> Signal<Option<SortKey>>`; search: `quick_filter() -> Signal<String>`; view: `visible() -> Signal<Rc<Vec<u32>>>`, `rows() -> Signal<Arc<Vec<Row>>>`; selection: `selection() -> Signal<Rc<HashSet<u32>>>`, `selected_index() -> Option<u32>`, `select_only(row)`, `select_by(delta: i32)`, `toggle_selected(row)`, `select_range_to(row)`; columns: `columns()`, `visible_columns()`, `hide_column(id)`, `show_all_columns()`, `move_column(from, to)`, `column_width(id) -> Option<f32>`, `pin_column_width(id, px)`, `clear_column_width(id)` | Rows, columns, sort, and selection. `identified` keeps the selection on the same row ids across `replace_rows`. `reset_columns` restores the original column order and shows every column. `row_eq` tells the table when a replaced row is unchanged, so its cells are not rebuilt; without it every replace rebuilds the mounted rows. `toggle_sort` steps a sortable column through ascending, descending, and no sort; set `sort_key` to sort directly. `cycle_sort` moves the sort to the next sortable column in column order, ascending, wrapping to the first; with no sort it picks the first sortable column. When the sort or `quick_filter` hides a selected row, the row leaves the selection, so `selected_row` only returns a visible row. `reverse_sort` flips the direction of the current sort and does nothing without one. `select_by` moves the selection `delta` rows in visible order from the last selected row, clamped to the visible rows, and selects only that row; with no selected row, a negative `delta` selects the last visible row and any other the first. Text in `quick_filter` keeps the rows that a `filterable` column matches. `visible` holds the row indexes after filter and sort. Row numbers in the selection methods are indexes into `rows`. `select_range_to` selects from the last selected row to `row` in visible order. `pin_column_width` clamps to the smallest column width, 48 px. |
| `TableColumn` | `TableColumn::new(id: &'static str, title)`; `sort_by(Fn(&Row) -> K)` with `K: Ord`, `sortable(Fn(&Row, &Row) -> Ordering)`, `filterable(Fn(&Row, &str) -> bool)`, `cell(Fn(&Row) -> N)` with `N: IntoNode`, `numeric()`, `align(ColAlign)`, `width(ColWidth)`, `min_width(px)` | One table column. `sort_by` and `sortable` make the header sort the rows; `filterable` makes the column match the text in `TableState::quick_filter`; `cell` renders the cell, else the table shows nothing for it. `numeric` right-aligns the column and uses tabular digits. |
| `ColWidth` | `enum ColWidth { Fixed(f32), Fraction(f32) }` | Column width. |
| `Density` | `enum Density { Grid, Compact, Default }` | Row density of a table. |
| `Identifiable` | `trait Identifiable { type Id; fn id(&self) -> Self::Id; }` | Stable row identity. |
| `virtual_list` | `fn virtual_list<T, R: Fn(usize, &T) -> N, N: IntoNode, I: IntoList<T, R>>(items: I, render: R) -> VirtualListBuilder<T, R>` | A list that builds only the visible rows. `render` returns any `IntoNode`, such as `text(..)` or a widget builder. |
| `SelectionMode` | `enum SelectionMode { None, Single, Multi, Extend }` | Selection mode of a list. |
| `list_row` | `fn list_row(selected: bool, title, slots: ListRowSlots) -> NodeBuilder` | One row of a list, with a marker, status, and columns. With `slots.activate` set, the row takes keyboard focus and runs the callback on activation. |
| `ListRowSlots` | `struct ListRowSlots { depth, disclosure, marker, marker_tone, leading, status, status_tone, badge, columns, shrink, title_width, indent, tone, activate: Option<Rc<dyn Fn()>> }` | Optional parts of a list row. `title_width` gives the title a fixed width so columns line up after it. `leading` places a node, such as an icon, before the title. `marker_tone` and `status_tone` color the marker and status text per row. `badge: Option<(TextContent, Tone)>` draws a count pill after the status. `indent` sets the columns per depth level (default 2). |
| `Disclosure` | `enum Disclosure { None, Leaf, Collapsed, Expanded }` | The tree chevron slot of a list row. |
| `kv_grid` | `fn kv_grid(rows: impl IntoIterator<Item = (impl Into<TextContent>, impl Into<TextContent>)>) -> NodeBuilder` | A two-column key/value grid. Keys and values can be reactive. |
| `kv_row` | `fn kv_row(label, value: impl IntoNode + 'static) -> NodeBuilder` | One key/value row. |
| `tabs` | `fn tabs()`; `.tab(label, component)`, `.active(Signal<usize>)` | Tabs. Each `tab` adds a label and its content component; the component builds once, so a tab keeps its state while another is shown. When tabs have content, the active tab's content fills the space left below the tab strip. |
| `code_edit` | `fn code_edit(lines: Signal<Vec<String>>, caret: Signal<(usize, usize)>)` | A code editor with built-in `text_edit` editing. |
| `highlight` | `fn highlight(language: &str, source: &str) -> Vec<Vec<Highlight>>` | Syntax marks per line, in the grapheme columns `code_edit(..).highlights(..)` takes, for the languages and aliases in `widgets/highlight/languages.toml`: rust, sh, toml, json, lean, python, javascript, typescript, go, c/cpp, yaml, css, html, sql, markdown, diff, java and the JVM languages, C#, Swift, Ruby, PHP, Lua, Zig, Haskell, OCaml, Elixir, R and Julia, Dart, PowerShell, Dockerfile, Makefile, Nix, HCL, protobuf and GraphQL, INI, and Perl. Other languages get no marks. |
| `markdown` | `fn markdown(source: Signal<String>) -> NodeBuilder` | Rendered markdown. Each link is exposed as a `Link` node with its URL and on-screen area, and a click on it reaches the nearest `link` handler. |
| `markdown_inline` | `fn markdown_inline(source: &str) -> TextBuilder` | One wrapped paragraph with inline markdown spans. |
| `markdown_edit` | `fn markdown_edit(source: Signal<String>)` | A markdown editor with preview. |
| `html` | `fn html(source: Signal<String>) -> NodeBuilder` | Rendered HTML subset. |
| `mermaid` | `fn mermaid(source: impl Reactive<String>) -> NodeBuilder` | A rendered mermaid diagram. A signal or closure source draws the diagram again when it changes. It draws as vector scene commands, so it stays sharp under zoom and a theme change lowers it again instead of rastering it. |
| `a2ui`, `A2uiBuilder` | `fn a2ui(json: impl Reactive<String>) -> A2uiBuilder`; `.activate(f: impl Fn(&str, &str) + 'static)` | An A2UI v0.9 basic-catalog surface (one message, an array, or `{"messages": [..]}`) rendered as native widgets from the component with id `root`. Inputs bind both ways to the data model by JSON Pointer path. A pressed Button calls `activate` with its action name and its context as JSON, each bound path resolved. Images load from base64 `data:` URIs only. Invalid, oversize or rootless input shows a warning notice and the JSON as code. |
| `graphviz` | `fn graphviz(source: impl Reactive<String>) -> NodeBuilder` | A Graphviz DOT diagram, laid out by layout-rs and drawn in theme colors as vector scene commands that stay sharp under zoom. It scrolls horizontally when wide. A terminal, or DOT that does not parse, shows the framed source. |
| `math` | `fn math(source: impl Reactive<String>) -> NodeBuilder` | A LaTeX formula typeset in display style in the theme text color, drawn as vector scene commands that stay sharp under zoom. A terminal, or a formula that does not parse, shows a Unicode approximation. The accessible description is the LaTeX source. |
| `diff` | `fn diff(patch: impl DiffSource + 'static) -> DiffBuilder`; `.select(Signal<Selection>)` | A rendered unified diff of text or a parsed `Patch`. A signal or closure source renders the rows again when it changes. Rows render through `virtual_list`, so a long patch builds only the visible rows. Code is colored by the language of the file path. With `select`, a click on a hunk header selects the hunk and a click or Shift+click selects changed lines, for staging. |
| `Patch`, `Hunk`, `Line` | `Patch::parse(&str) -> Patch`; `struct Patch { hunks, trailer }`, `line(LineId)`, `changes()`, `hunk_changes(usize)`; `struct Hunk { meta, path, header, old_start, new_start, lines }`; `struct Line { kind, text, old, new }`; `enum LineKind { Context, Removed, Added, Note }` | The parsed unified diff that `diff` renders. |
| `Selection` | `struct Selection { hunks: BTreeSet<usize>, lines: BTreeSet<LineId>, .. }`, `contains`, `select_hunk`, `select_line`; `trait DiffSource` for text, `Patch`, and `Signal<Patch>` | What the user selected in a `diff`, and the sources `diff` accepts. |
| `DiffView` | `enum DiffView { Split, Unified }` | Diff view: side-by-side or unified rows. Split columns have equal width independent of line lengths, so adjacent diffs keep the same divider position. |
| `terminal` | `fn terminal(lines: Signal<Vec<String>>, on_input: impl Fn(String) + 'static) -> NodeBuilder` | A terminal transcript with an input line. |

```rust,no_run
use sigil::widgets::ColAlign;
use sigil::widgets::DiffBuilder;
use sigil::widgets::Highlight;
use sigil::widgets::ScrollAnchor;
use sigil::widgets::SelectionState;
use sigil::widgets::VirtualListBuilder;
use sigil::prelude::*;

#[derive(Clone)]
struct File { name: String, size: u64 }

fn tree() -> impl IntoNode {
    let _ = dropdown(signal(0), vec!["One".into(), "Two".into()]);
    let _ = combobox(Combobox::new(vec!["red".into(), "blue".into()]));
    let _ = menu(signal(false));
    let _: Option<MenuItem> = None;
    let _ = (menu_bar(), command_menu());
    let _ = fuzzy_score("sv", "save file").map(|found| found.score);
    let size = TableColumn::new("size", "Size").numeric().width(ColWidth::Fixed(80.0)).align(ColAlign::End);
    let columns = vec![TableColumn::new("name", "Name").cell(|f: &File| text(f.name.clone()).nowrap()), size];
    let files = table(TableState::new(vec![File { name: "a".into(), size: 1 }], columns));
    let _ = (Density::Compact, ScrollAnchor::Bottom, SelectionState::new(SelectionMode::Single));
    let list: VirtualListBuilder<String, _> = virtual_list(vec!["a".to_string()], |_, s: &String| list_row(false, s.clone(), ListRowSlots::default()));
    let (scroll, max_scroll) = (signal(0.0_f32), signal(0.0_f32));
    let list = list.measured().estimate(rows(2.0)).scroll_bind(scroll).max_scroll_bind(max_scroll);
    let list = list.content_height_bind(signal(0.0)).follow_row(signal(0));
    let _ = list_row(true, "Inbox", ListRowSlots { marker: Some("*".into()), ..ListRowSlots::default() });
    let _ = (kv_grid([("Name", "Sigil")]), kv_row("Size", text("1 KB")));
    let _ = tabs().tab("A", || text("a")).tab("B", || text("b"));
    let _ = code_edit(signal(vec!["fn main() {}".into()]), signal((0, 0)));
    let _ = Highlight { start: 0, end: 2, token: Token::Accent };
    let _: Vec<Vec<Highlight>> = highlight("rust", "fn main() {}");
    let _ = code_edit(signal(vec!["let x = 1;".into()]), signal((0, 0))).highlights(signal(highlight("rust", "let x = 1;")));
    let doc = signal("# Title".to_string());
    let _ = (markdown(doc), markdown_edit(doc), html(signal("<b>x</b>".into())), mermaid("graph TD; A-->B"));
    let _: DiffBuilder = diff("--- a\n+++ b\n").view(DiffView::Unified);
    let _ = terminal(signal(Vec::new()), |line| println!("{line}"));
    fn _id<T: Identifiable>() {}
    container().child(files).child(list)
}

fn main() {
    let _ = tree;
}
```

### Widgets: frame, overlays, and display

| Name | Signature | Purpose |
| --- | --- | --- |
| `app_shell` | `fn app_shell(top, work, status, modal: Option<impl IntoNode>) -> NodeBuilder` | Top bar, work area, status bar, and modal slot. |
| `sidebar` | `fn sidebar(child: impl IntoNode + 'static)` | A side column. Set its width with `.width(..)`. |
| `status_bar` | `fn status_bar<I: IntoIterator>(children: I) -> NodeBuilder` | A status line. |
| `panel` | `fn panel()` | A titled panel. |
| `card` | `fn card(child: impl IntoNode + 'static)`; `.selected(impl Reactive<bool>)`, `.activate(f)` | A bordered card. `selected` gives it the accent border and background; `activate` handles click, Enter, and Space and makes the card a focusable button. Converts into `NodeBuilder`. |
| `stat` | `fn stat(label, value)`; `.delta(text, Tone)`, `.caption(text)`, `.trend(values)`, `.range(low, high)`, `.tone(Tone)` | A KPI tile in card chrome: a label, a large value, and an optional toned change line, caption, and sparkline trend. Label and value can be reactive. |
| `tree` | `fn tree<K: Clone + Eq + Hash + 'static>(nodes: impl Reactive<Vec<TreeNode<K>>>)` | A tree view for explorers, outlines, and settings. Only rows under expanded nodes are built, and they are virtualized. Click toggles a folder and activates a leaf; arrows move, Left and Right collapse and expand, Home and End jump, Enter activates. |
| `sheet` | `fn sheet(show: Signal<bool>) -> ModalBuilder` | A panel that slides in from a side. |
| `modal` | `fn modal(show: Signal<bool>) -> ModalBuilder` | A modal dialog. While it is open and traps focus, it is marked modal and the rest of the page is hidden from assistive technology. |
| `ModalBuilder` | `struct ModalBuilder`; `close_button(bool)`; `ModalBuilder::new(Signal<bool>)`, `title(text)`, `child(impl IntoNode)`, `footer(impl IntoNode)`, `actions(cancel, confirm)`, `side(Side)`, `escape_closes(bool)`, `dismiss_on_backdrop(bool)`, `trap(bool)`; width with `Widget::width` | Builder of a modal or sheet. A titled dialog shows a close button that sets `show` to false; `close_button(false)` removes it. Children scroll in a body between the title and the footer, so the title and actions stay visible when the dialog reaches its maximum height. `footer` adds one node to the footer; `actions(cancel, confirm)` adds two. Escape closes the dialog by default. A click on the backdrop closes a `sheet` but not a `modal`, unless `dismiss_on_backdrop` changes it. `trap(false)` lets focus leave the dialog. |
| `SortDirection` | `#[non_exhaustive] enum SortDirection { Ascending, Descending }` | Direction of a table sort. |
| `SortKey` | `struct SortKey { column: &'static str, dir: SortDirection }` | Table sort column and direction. |
| `Side` | `enum Side { Left, Right, Bottom }` | Edge of a sheet. |
| `splitter` | `fn splitter(orientation: Orientation)` | Resizable panes. |
| `scroll` | `fn scroll(content: NodeBuilder)` | A scroll area. It measures its content; `.extent(f32 or Signal<f32>)` sets a known content height instead. |
| `section_label` | `fn section_label(title) -> NodeBuilder` | A section heading row. |
| `hint_bar` | `fn hint_bar(keys, right) -> NodeBuilder` | A key hint row. |
| `hint_overlay` | `fn hint_overlay(hints: &[(&str, &str)])` | An overlay that lists key bindings. |
| `notice_band` | `fn notice_band(body, tone: Tone) -> NodeBuilder` | A full-width notice. |
| `empty_state` | `fn empty_state(title, body) -> NodeBuilder` | A placeholder for empty content. |
| `spacer` | `fn spacer() -> NodeBuilder` | Flexible empty space. |
| `divider_horizontal` | `fn divider_horizontal() -> NodeBuilder` | A horizontal divider. |
| `divider_vertical` | `fn divider_vertical() -> NodeBuilder`; `.height(px)` from `Widget` | A vertical divider that fills the height. `height` gives a fixed tick, such as between toolbar items. |
| `surface` | `fn surface(key: u64) -> NodeBuilder` | A rectangle that the embed host draws. |
| `tooltip` | `fn tooltip(label: impl Into<String>)` | A hover tooltip. |
| `toast` | `fn toast(message: Signal<Option<String>>) -> NodeBuilder` | One transient message. |
| `toast_queue` | `fn toast_queue(queue: ToastQueue) -> NodeBuilder` | A stack of transient messages. |
| `ToastQueue` | `ToastQueue::new()`; `info`, `success`, `warning`, `error` | Queue of toast messages. |
| `progress` | `fn progress(value: Signal<f32>, max: f32)`; `.label(text)` | A progress bar. `.label` shows the text beside the bar and names it; without a label it is named "Progress". |
| `gauge` | `fn gauge(value: Signal<f32>)` | A labelled meter bar that shows a value against a maximum. |
| `chart` | `fn chart(values: impl Reactive<Vec<f32>>) -> ChartBuilder`; `ChartBuilder::candles(rows: impl Reactive<Vec<(f32, f32, f32, f32)>>)`, `.x_range(Signal<Range<f32>>)`, `.crosshair()` | A bar, line, or candle chart. A line with more points than the plot has pixel columns keeps the low and high point of each column, so 100k points draw as one short path. `candles` takes `(open, high, low, close)` rows. `x_range` binds the visible range in point units: the wheel zooms around the pointer and a drag pans. `crosshair` draws rules and the value under the pointer. A signal or closure source draws the chart again when it changes, like `sparkline`. The chart fills its parent's width unless `.width()` is set; at small heights it thins x-axis labels and hides the axis row before the plot gets too small. |
| `sparkline` | `fn sparkline(values: impl Reactive<Vec<f32>>) -> SparklineBuilder` | A sparkline. By default, one block glyph per value on one text row. A signal or closure source draws the graph again when it changes. |
| `heatmap` | `fn heatmap<F: Fn(usize, usize) -> Rgba8>(grid_w, grid_h, color: F) -> NodeBuilder` | A grid of colored cells, drawn as one image that fills its box. |
| `dot` | `fn dot(tone: Tone)` | A status dot. |
| `avatar` | `fn avatar(initials) -> NodeBuilder` | Initials in a circle. |
| `spinner` | `fn spinner() -> NodeBuilder` | A busy indicator named "Loading". |
| `icon` | `fn icon(glyph: impl IntoIconSource) -> IconBuilder` | An icon: a bundled `Icon`, such as `icon(Icon::Folder)`, drawn as vector paths, or a text glyph. |
| `Icon` | `enum Icon` (108 variants); `Icon::ALL`, `name()`, `svg()` | The bundled Lucide icon set (ISC). `name` is the kebab-case file name and `svg` the SVG source (feature `scene`). |
| `path` | `const fn path(width: u32, height: u32) -> PathNode` | A vector path. |
| `widgets::canvas` | `fn canvas(record: impl Fn(&mut SceneBuilder, Rect) + 'static)`; `redraw_on(Signal<u64>)`, `layout(LayoutStyle)` | Custom drawing: the closure records scene commands for the node's rect when the node paints, clipped to that rect. It runs again on resize and each time the `redraw_on` signal changes. The terminal host paints the same commands to cells; a fill that covers part of a cell paints as a half or eighth block. |
| `widgets::PathNode::draw` | `fn draw(self, scene: &mut SceneBuilder, destination: Rect, theme: &Theme)` | Record a vector path into a `canvas` scene. It records one vector path command, so the path stays sharp at any scale. |
| `PathNode` | `move_to`, `line_to`, `quad_to`, `cubic_to`, `close`, `fill`, `fill_linear(start, end, stops: Vec<(f32, Rgba8)>)`, `fill_radial(center, radius, stops)`, `even_odd`, `stroke`, `cap(StrokeCap)`, `join(StrokeJoin)`, `dash(&[f32])`, `non_scaling_stroke` | Builder of a vector path. `fill_linear` and `fill_radial` fill with a gradient. `even_odd` fills with the even-odd rule; the default is nonzero. `cap` and `join` default to butt and miter (miter limit 4). `dash` takes alternating on and off lengths. The GPU and CPU renderers draw the path as vectors at the device scale. `non_scaling_stroke` keeps the stroke width in logical px under any fit, with round joins and caps unless `cap` or `join` is set. |
| `scene::StrokeCap` | `enum StrokeCap { Butt, Round, Square }` | The shape at the open ends of a stroked path. |
| `scene::StrokeJoin` | `enum StrokeJoin { Miter, Round, Bevel }` | The shape where two stroked segments meet. |

```rust,no_run
use sigil::widgets::ChartBuilder;
use sigil::widgets::IconBuilder;
use sigil::widgets::icon_svg;
use sigil::widgets::spinner_frame;
use sigil::widgets::ModalBuilder;
use sigil::widgets::SparkMark;
use sigil::widgets::SparklineBuilder;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let open = signal(false);
    let queue = ToastQueue::new();
    queue.info("Saved");
    let dialog: ModalBuilder = modal(open);
    let _: ModalBuilder = sheet(open);
    let _ = Side::Right;
    let _ = splitter(Orientation::Horizontal);
    let _ = scroll(container()).extent(400.0_f32);
    let _ = panel();
    let _ = hint_overlay(&[("q", "quit")]);
    let _ = (section_label("Files"), hint_bar("q quit", "v0.1"), notice_band("Offline", Tone::Warn));
    let _ = (empty_state("No files", "Add one"), divider_horizontal(), divider_vertical());
    let _ = (surface(1), toast(signal(None)), toast_queue(queue), Tone::Error);
    let _ = tooltip("Help");
    let _ = (progress(signal(0.3), 1.0).tone(Tone::Warn), gauge(signal(0.5)));
    let _: ChartBuilder = chart([1.0, 2.0, 3.0]).line();
    let _: SparklineBuilder = sparkline([1.0, 4.0, 2.0]).range(0.0, 5.0).tone(Tone::Warn);
    let _: SparklineBuilder = sparkline([1.0, 4.0]).mark(SparkMark::Braille).rows(3).tones([None, Some(Tone::Error)]);
    let _ = heatmap(4, 4, |x, y| Rgba8 { r: (x * 60) as u8, g: (y * 60) as u8, b: 0, a: 255 });
    let _ = dot(Tone::Success);
    let _ = (avatar("TG"), spinner(), spinner_frame(3));
    let _: IconBuilder = icon("*");
    let _: IconBuilder = icon_svg("<svg xmlns='http://www.w3.org/2000/svg'/>").tone(Tone::Primary);
    let _: PathNode = path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0).stroke(sigil::style::Token::Gray11, 1.5);
    let _: PathNode = path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0).close().fill(Rgba8 { r: 0, g: 0, b: 0, a: 255 });
    let _: Option<ImageBuilder> = ImageSource::from_rgba8(1, 1, vec![0, 0, 0, 255]).map(image);
    let _ = spacer();
    let work = sidebar(card(text("Card"))).width(200.0);
    app_shell(text("Top"), work, status_bar([text("Ready")]), Some(dialog))
}

#[cfg(feature = "image-decode")]
fn decoded(bytes: &[u8]) -> Option<ImageBuilder> { ImageSource::decode(bytes).map(image) }

fn main() {
    let _ = tree;
}
```

### Drag and drop

| Name | Signature | Purpose |
| --- | --- | --- |
| `Widget` drag and drop | `fn drag_source(self, state, payload)`, `fn drop_target(self, state, accepts, on_drop)` | Make any builder a drag source or a drop target. |
| `DragState` | `DragState::new()` | Shared state of one drag interaction. |
| `drag_ghost` | `fn drag_ghost<T, N>(state: DragState<T>, render: impl Fn(&T) -> N + 'static) -> NodeBuilder` | The node that follows the pointer. |
| `axis_drag` | `fn axis_drag<S>(node, axis: Orientation, snapshot, on_delta, on_end) -> NodeBuilder` | Drag along one axis. |

```rust,no_run
use sigil::prelude::*;
use sigil::widgets::Orientation;

fn tree() -> impl IntoNode {
    let drag: DragState<u32> = DragState::new();
    let source = container().drag_source(drag.clone(), 7);
    let target = container().drop_target(drag.clone(), |n: &u32| *n > 0, |n| println!("{n}"));
    let ghost = drag_ghost(drag, |n: &u32| text(n.to_string()));
    let width = signal(200.0_f32);
    let handle = axis_drag(container(), Orientation::Horizontal, move || width.peek(), move |start, dx| width.set(start + dx), || {});
    container().child(source).child(target).child(ghost).child(handle)
}

fn main() {
    let _ = tree;
}
```

## Module items

Public items outside the prelude. Import them by path.

| Name | Signature | Purpose |
| --- | --- | --- |
| `reactive::timeout` | `fn timeout(delay: Duration, callback: impl FnOnce() + 'static) -> IntervalId` | Run `callback` once after `delay`. Cancelled with its owner or by `IntervalId::cancel`. |
| `AppConfig::inline` | `const fn inline(self, yes: bool) -> Self` | Run the terminal host in the normal screen below the prompt instead of the alternate screen. |
| `AppConfig::modify_other_keys` | `const fn modify_other_keys(self, yes: bool) -> Self` | Terminal host. Turn on xterm modifyOtherKeys level 2 when the terminal lacks the kitty keyboard protocol, so Shift+Enter and Ctrl+letter arrive as their own keys. Off by default; reset on exit, suspend, and panic. Needs a crossterm that decodes `CSI 27 ; mod ; code ~`. |
| `AppConfig::reduced_motion` | `const fn reduced_motion(self, reduced: bool) -> Self` | Override the OS reduced-motion setting that `reduced_motion()` reports. |
| `AppConfig::double_click_time` | `const fn double_click_time(self, time: Duration) -> Self` | Set the double-click time. Default 450 ms. `sigil::run` uses the OS value on macOS and GNOME when this is unset. |
| `AppConfig::double_click_distance` | `const fn double_click_distance(self, distance: f32) -> Self` | Set the double-click distance in pixels. Default 8. |
| `AppConfig::task_workers` | `const fn task_workers(self, count: NonZeroUsize) -> Self` | Fix the worker threads of the task runtime. The default is one per CPU on every host. |
| `AppConfig::terminal_hooks` | `fn terminal_hooks(self, hooks: terminal::Hooks<'static>) -> Self` | Terminal host hooks for `sigil::run`: `external_events`, `after_frame`, `region_rows`. The window host ignores them. |
| `terminal::Hooks::region_rows` | `region_rows: Option<Box<dyn FnMut() -> u16>>` | Height of the inline live region each frame, clamped to 1..=terminal rows. Used only with `inline(true)`. |
| `terminal::print_above` | `fn print_above(text: &str) -> Result<(), Error>` | Write lines above the inline live region, into scrollback. Returns an `io::Error` that wraps a `sigil::Error` with kind `ErrorKind::NotLive` when no host is live on this thread, or `ErrorKind::NotInline` when the host is not inline. |
| `AppConfig::terminal_access` | `const fn terminal_access(self, access: terminal::Access) -> Self` | Screen-reader options for the terminal host. The window host ignores them. |
| `terminal::Access` | `struct Access { focus_cursor: FocusCursor, status: Duration }`; `Default` is `Follow` and 5 s | Where the terminal cursor goes when no caret shows, and how long an announcement stays on the full-screen status row. `Screen::set_access(access)` applies it to a `Screen`. |
| `terminal::FocusCursor` | `enum FocusCursor { Follow, Off }` | `Follow` moves the hidden terminal cursor to the focused node's label when no text caret is visible, so screen readers read it. `Off` leaves it where it is. The cursor shows only at a text caret. |
| `AppConfig::mouse_tracking` | `const fn mouse_tracking(self, tracking: terminal::MouseTracking) -> Self` | Mouse reports the terminal host asks for at start. Without it, the host keeps the last `set_mouse_tracking` value, `Drags` by default. The window host ignores it. |
| `terminal::MouseTracking` | `enum MouseTracking { Off, Clicks, Drags, Motion }`; `Default` is `Drags` | Terminal mouse reports, all in SGR form (1006). `Clicks` sends presses, releases, and the wheel (1000). `Drags` adds motion while a button is held (1002). `Motion` adds motion with no button held (1003), for hover. Every change and the exit first reset all four modes. |
| `AppConfig::resizable` | `const fn resizable(self, yes: bool) -> Self` | Let the user resize the window (feature `desktop`). `false` fixes the window at its configured size. |
| `AppConfig::esc_exits` | `const fn esc_exits(self, yes: bool) -> Self` | Exit the app on Escape. |
| `AppConfig::suspend_on` | `fn suspend_on(self, shortcut: impl Into<Option<Shortcut>>) -> Self` | The key that suspends the terminal host (job control); the default is Ctrl+Z. The host restores the terminal and stops; `fg` resumes it and redraws the screen. `None` turns it off and delivers the key to the app. Unix only; the window host ignores it. |
| `Cx::flush_announcements` | `fn flush_announcements(&mut self)` | Move the text that `announce` queued into the live region. `Host::frame` calls it every frame. |
| `announce` | `fn announce(text: impl Into<String>, politeness: Live)` | Queue a screen-reader announcement; the next frame posts it to the live region. `Live::Assertive` interrupts the reader. |
| `Cx::dump_tree` | `fn dump_tree(&self, root: NodeId) -> String` | Indented text of the node tree below `root`, for debugging. |
| `Cx::describe` | `fn describe(&self, id: NodeId) -> String` | A short label for one node: its role, then its accessible name, its text, or the nearest named ancestor. Profiler attributions use it. |
| `app::menu::Menu::install` | `fn install(self) -> AppResult<()>` | Queue the native menu bar. The window host installs it on the next frame (feature `menu`). Returns an error on a platform with no native menu bar. A shortcut with no native key keeps its item, drops the accelerator, and logs a warning. |
| `app::tray::Tray::install` | `fn install(self) -> AppResult<()>` | Queue a system tray icon. The window host builds it on the next frame (feature `tray`). Returns an error when the icon image is invalid. |
| `scene::SceneBuilder::with_clip` | `fn with_clip(&mut self, clip: impl Into<Clip>, body: impl FnOnce(&mut Self))` | Clip the commands `body` records to `clip`. A `Rect` clips through the scissor with no layer; a rounded or path clip draws `body` into a layer and composites it through the clip's coverage. Nested clips intersect. |
| `scene::Clip` | `enum Clip { Rect(Rect), Rounded(Rect, CornerRadii), Path(DrawPath) }`; `Clip::rounded(rect, radius)`, `Clip::circle(center, radius)`, `Clip::ellipse(rect)`, `Clip::polygon(&[Point])`; `From<Rect>` | The region a clip keeps. Rounded rects and circles composite through a signed distance field; ellipses, polygons and paths through a coverage mask drawn by vello. The terminal host clips to the clip's bounding rect. |
| `scene::SceneBuilder::with_layer` | `fn with_layer(&mut self, opacity: f32, body: impl FnOnce(&mut Self))` | Draw `body` into a layer composited at `opacity`. |
| `scene::SceneBuilder::with_transform` | `fn with_transform(&mut self, transform: Transform, body: impl FnOnce(&mut Self))` | Draw `body` under `transform`. |
| `app::dialog::FileDialog` | `FileDialog::new()`; `filter(name, extensions)`, `directory(dir)`, `file_name(name)`; `open_file()`, `save_file()`, `pick_folder() -> Picked<PathBuf>`, `open_files() -> Picked<Vec<PathBuf>>` | File picker (feature `dialog`). The result signal starts `Pending` and resolves without blocking the frame loop. The in-app picker runs on every host; `native-dialog` opens the OS dialog on the window host. |
| `LayoutStyle` | `grid_columns(Vec<TrackSize>)`, `grid_rows(Vec<TrackSize>)`, `grid_area(col, row)`, `grid_dense()`, `max_height(impl Into<Dimension>)` | Grid tracks and placement. |
| `style::Metrics::icon_size` | `fn icon_size(self) -> f32` | Icon size in points, or one cell on the terminal host. |

| `app::load_font_url` | `fn load_font_url(url: &str, face_index: u32) -> Signal<Status<(), Error>>` (wasm32 only) | Fetch a font file and add it as a fallback face. The signal turns `Ok(())` when the face is in use. |
| `Dimension` | `enum Dimension { Px(f32), Percent(f32), Auto, Fill, Rows(f32), Cols(f32), Em(f32) }` | The length of a size, padding, margin, gap, or inset. `rows`, `cols`, and `em` return it. |

| `clipboard::paste` | `fn paste() -> AppResult<String>` | Read text from the installed clipboard. |

## Module `animation`

| Name | Signature | Purpose |
| --- | --- | --- |
| `animation::Easing` | `enum Easing` | Tween easing curve, with `CubicBezier` and the Tailwind `IN`, `OUT`, and `IN_OUT` curves. |
| `animation::Keyframes` | `struct Keyframes` | A CSS keyframe animation for `Paint::animate`. |
| `animation::Property` | `enum Property` | A property value that a keyframe sets. |
| `animation::reduced_motion` | `fn reduced_motion() -> bool` | Reduced-motion preference. `sigil::run` sets it from `AppConfig::reduced_motion`, else from the OS (macOS accessibility setting, GNOME `enable-animations`). |

## Module `app`

| Name | Signature | Purpose |
| --- | --- | --- |
| `app::menu::Menu`, `app::menu::StandardItem` | `Menu::new()`, `.item(id, label, Option<Shortcut>)`, `.disabled(id, label)`, `.check(id, label, checked, Option<Shortcut>)`, `.standard(StandardItem)`, `.separator()`, `.submenu(label, Menu)`, `Menu::events()`; `#[non_exhaustive] enum StandardItem` | A native menu. `Menu::events` delivers the id of each item the user picks as a `String`. |
| `app::tray::Tray`, `app::tray::TrayEvent` | `Tray::new()`, `.icon(rgba, width, height)`, `.tooltip(text)`, `.title(text)`, `.menu(Menu)`, `.clear_icon()`, `.clear_tooltip()`, `.clear_title()`, `.clear_menu()`, `update(&self) -> AppResult<bool>`, `Tray::events()`; `#[non_exhaustive] enum TrayEvent { Click { button, position }, DoubleClick { button, position }, Enter, Leave }` | A system tray icon and its pointer events, which arrive on `Tray::events`. `update` applies the set and cleared fields to the installed icon and leaves the others alone. It returns `Ok(false)` before the icon is installed, and an error when an icon or menu is invalid or the platform refuses the change. |
| `app::cell_width` | `fn cell_width(text: &str) -> usize` | Set the cell width rule for this thread, and return the number of terminal cells that a string occupies under that rule. |
| `app::FrameInfo` | `#[non_exhaustive] pub struct FrameInfo { dirty: bool, animating: bool, layout_ok: bool }` | State of the current frame that `Host::frame` passes to the paint hook. |
| `app::FrameHooks` | `struct FrameHooks<'a> { input: Option<FrameInputHook<'a>>, after_layout: Option<FrameLayoutHook<'a>>, paint: Option<FramePaintHook<'a>> }`; `type FrameInputHook<'a> = Box<dyn FnOnce(&mut Host) -> Option<(f32, f32)> + 'a>`; `type FrameLayoutHook<'a> = Box<dyn FnOnce(&mut Host) + 'a>`; `pub type FramePaintHook<'a> = Box<dyn FnOnce(&mut Host, &FrameInfo) + 'a>` | Callbacks that an embedding host passes to `Host::frame` to feed input, act after layout, and paint. The input hook returns the viewport when input changed it (a resize), and the frame lays out at that size; `None` keeps the viewport passed to `frame`. |
| `app::is_grid` | `fn is_grid() -> bool` | `true` on a cell grid (terminal) host. |
| `app::viewport_size` | `fn viewport_size() -> Signal<LogicalSize>` | The viewport size in logical px. A terminal cell is `Host::CELL` px. |
| `app::set_theme` | `fn set_theme(theme: Theme)` | Replace the active theme at the next frame. |
| `Theme: FromStr` | `fn from_str(input: &str) -> Result<Theme, Error>` | Parse a theme TOML document. |
| `app::watch_theme` | `fn watch_theme(path: impl Into<PathBuf>) -> AppResult<ThemeWatcher>` | Reload a theme file when it changes. |
| `app::ThemeWatcher` | `struct ThemeWatcher` | Guard of `watch_theme`. Drop it to stop the watch. |
| `app::open` | `fn open(target: impl AsRef<OsStr>) -> AppResult<()>` | Open a file or URL with the OS handler (feature `desktop`). |
| `app::dialog` | `mod dialog` | File and message dialogs. No call blocks the frame loop. Feature `dialog` gives the API and an in-app modal that works on every host and needs no desktop dependencies. Feature `native-dialog` adds the OS dialog on the window host, run on the task runtime; the terminal host, or a Linux session without `DISPLAY` or `WAYLAND_DISPLAY`, still uses the in-app modal. The in-app modal shows the directory path, a list with `..` and folders first, the extension filter applied, a name field in save mode, and Cancel and Open buttons. Arrows move, Enter opens a folder or picks a file, Backspace goes up, and Escape cancels. The caller code is the same on both hosts. |
| `app::tray` | `mod tray` | System tray icon (feature `tray`). |
| `app::notify`, `app::notify::Notification` | `struct Notification { title, body, app_name }`, `Notification::new(title, body)`, `.app_name(..)`, `send(&self) -> AppResult<()>` | Desktop notification on macOS, Windows, and freedesktop systems. Other targets return an unsupported error (feature `notify`). |
| `app::media`, `app::media::now_playing`, `app::media::events` | `fn now_playing(Track) -> AppResult<()>`; `fn events() -> EventChannel<MediaKey>`; `struct Track` (title, artist, album, cover, duration, position, state, `shuffle`, `repeat`), `enum PlaybackState`, `enum Repeat { Off, One, All }`, `enum MediaKey` | Now-playing information and media keys through the OS media controls: MPNowPlayingInfoCenter, SMTC, MPRIS, by the `playwire` crate (feature `media`). Shuffle and repeat changes from the OS arrive as `MediaKey::SetShuffle` and `MediaKey::SetRepeat`. |
| `app::hotkey`, `app::hotkey::register`, `app::hotkey::unregister` | `fn register(&Shortcut) -> AppResult<EventChannel<KeyPhase>>`; `fn unregister(&Shortcut) -> AppResult<bool>` | A system-wide hotkey that fires while the app is in the background. Register from the UI thread (feature `hotkey`). |
| `app::menu` | `mod menu` | Native menu bar (feature `menu`). |
| `app::Player` | `struct Player`; `play_path`, `play_bytes(Vec<u8>)`, `queue(paths)`, `append(path)`, `stop()`, `toggle()`, `seek`; `volume: Signal<f32>`, `duration: Signal<Option<Duration>>` | Audio player (feature `audio`). It keeps one output device for its life, so `append` and `queue` play tracks back to back with no gap. Setting `volume` changes the output level. |
| `app::Session` | `struct Session` | Child process in a pseudo-terminal (feature `pty`). |
| `ErrorKind` | `#[non_exhaustive] enum ErrorKind` | Class of an `Error`. One kind enum for the whole crate. |
| `app::PanicPolicy` | `enum PanicPolicy { Isolate, Abort }` | What a panic in a handler does. |
| `Host::set_hyperlinks` | `const fn set_hyperlinks(&mut self, yes: bool)` | Whether the host can show hyperlinks. The terminal host sets it from the detected terminal. |
| `app::WindowCommand` | `enum WindowCommand { Close, Minimize, ToggleMaximize, Drag, DragResize, SetZoom, Show, Hide, Focus, SetTitle(String), SetPosition(Point), SetLevel(WindowLevel) }` | Commands for the window whose frame runs, or for a `WindowId` through `WindowId::command`. Feature `desktop`. |
| `app::WindowLevel` | `enum WindowLevel { AlwaysOnBottom, Normal, AlwaysOnTop }` | Window stacking level. |
| `app::CloseAction`, `AppConfig::on_close` | `enum CloseAction { Close, Hide, Keep }`; `fn on_close(self, decide: impl Fn() -> CloseAction + 'static) -> Self` | What a close request does: close the window (the default), hide it and keep the app running (for tray apps), or ignore it. |
| `AppConfig::on_exit` | `fn on_exit(self, run: impl Fn() + 'static) -> Self` (feature `desktop`) | Runs once on the UI thread when the window event loop exits: after the last window closes, on a macOS quit (Quit menu, Dock Quit, logout), and on a Windows session end (sign-out, shutdown, restart, or a Restart Manager close). A system quit or session end may not return from `run`, so work that must finish before the process ends goes here. On Windows the shell registers the `StringKey::SavingOnExit` text as the shutdown block reason while the hook runs, then ends the event loop. |
| `app::ResizeDirection` | `enum ResizeDirection` | Edge or corner for `WindowCommand::DragResize`. Feature `desktop`. |
| `app::Anchor` | `enum Anchor { BottomCenter { margin: f32 } }` | Screen anchor of an overlay window. Feature `desktop`. |
| `AppConfig::font_sans`, `font_mono`, `font_icon`, `font_display`, `font_fallback`, `font_sans_weight` | `fn font_sans(self, bytes: impl Into<Vec<u8>>, face: u32) -> Self`; `fn font_sans_weight(self, weight: u16, bytes: impl Into<Vec<u8>>, face: u32) -> Self` | Replace the bundled fonts when `sigil::run` starts. `font_fallback` adds a face to the fallback chain and keeps the system or bundled primary fonts. Feature `scene`. |
| `AppConfig::text_enhance`, `text_snap`, `text_raster`, `text_dilation`, `text_sharpen`, `text_gamma` | `fn text_raster(self, yes: bool) -> Self`; `fn text_dilation(self, dilation: f32) -> Self`; `fn text_gamma(self, light_on_dark: f32, dark_on_light: f32) -> Self` | Text rasterization settings, applied when `sigil::run` starts. Feature `scene`. |

## Module `clipboard`

| Name | Signature | Purpose |
| --- | --- | --- |
| `clipboard::copy` | `fn copy(text: &str) -> AppResult<Delivery>` | Write text to the installed clipboard. |
| `clipboard::image`, `clipboard::copy_image` | `fn image() -> AppResult<Option<ImageSource>>`; `fn copy_image(image: &ImageSource) -> Result<Delivery, Error>`; `Clipboard::set_image` (feature `clipboard-image`) | Read an image from the clipboard, `None` when it holds no image, or write one. The web clipboard has no image write and returns an `Error` with kind `ErrorKind::Io`. |
| `Error::message` | `Error::message(kind: ErrorKind, message: impl Display) -> Error` | Build an error with a kind and a message, for example from a `Clipboard` back end with `ErrorKind::Clipboard`. |
| `clipboard::OsClipboard` | `struct OsClipboard` | The OS clipboard (feature `clipboard`). |
| `clipboard::install` | `fn install(clip: impl Clipboard + 'static)` | Replace the installed clipboard. |
| `clipboard::Clipboard` | `trait Clipboard { fn set(&self, &str) -> Result<Delivery, Error>; fn get(&self) -> Option<String>; }` | A clipboard back end. |
| `clipboard::Delivery` | `enum Delivery { Confirmed, Requested }` | Whether the OS took the text. |

```rust,no_run
use sigil::{Error, ErrorKind};
# #[cfg(feature = "clipboard")]
use sigil::clipboard::OsClipboard;
use sigil::clipboard::copy;
use sigil::prelude::*;
use sigil::clipboard::Clipboard;
use sigil::clipboard::Delivery;
use sigil::clipboard::install;

struct Memory;

impl Clipboard for Memory {
    fn set(&self, _text: &str) -> Result<Delivery, Error> { Err(Error::message(ErrorKind::Clipboard, "read-only")) }
    fn get(&self) -> Option<String> { Some("text".into()) }
}

fn main() -> AppResult<()> {
    install(Memory);
    let _: Delivery = copy("hello")?;
    let _: String = sigil::clipboard::paste()?;
#   #[cfg(feature = "clipboard")]
    let _: Option<OsClipboard> = None;
    Ok(())
}
```

## Module `events`

| Name | Signature | Purpose |
| --- | --- | --- |
| `events::FocusIntent` | `enum FocusIntent { Up, Down, Left, Right }` | Directional focus move. |
| `events::request_window_command` | `fn request_window_command(command: WindowCommand)` | Send a command to the window. Feature `desktop`. |
| `events::PhysicalKey` | `#[non_exhaustive] enum PhysicalKey` | A key position on the keyboard. |
| `events::ImeEvent` | `enum ImeEvent { Enabled, Preedit, Commit, Disabled }` | Input method event. |
| `events::ScrollEvent` | `struct ScrollEvent` | A wheel or trackpad scroll. Each `on_scroll` handler sees `position` relative to its own node, as pointer handlers do. |
| `events::ScrollDelta` | `enum ScrollDelta { Pixels(Point), Lines(Point) }` | Scroll amount. |
| `events::ScrollPhase` | `enum ScrollPhase { Started, Moved, Ended, Cancelled }` | Scroll gesture phase. |
| `events::FileDropPhase` | `enum FileDropPhase { Hovered, Dropped, Cancelled }` | File drop phase. |
| `events::WindowEvent` | `enum WindowEvent` | Resize, focus, and close events of the window. |
| `events::Event` | `enum Event { Pointer, Key, Ime, Scroll, Gesture(GestureEvent), FileDrop, Paste(String), Window }` | An input event from the host. The terminal host sends a bracketed paste as one `Paste`. |
| `Event` constructors | `key(key, phase)`, `key_down(key)`, `press(key, modifiers) -> [Event; 2]`, `typed(text)`, `type_text(text) -> Vec<Event>`, `tab(shift)`, `pointer(phase, position)`, `pointer_move(position)`, `pointer_drag(position)`, `pointer_down(position)`, `pointer_up(position)`, `click(position) -> [Event; 3]`, `scroll(position, delta, phase)`, `wheel(position, lines)`, `gesture(kind, phase, position)`, `paste(text)`, `blur()` | Build an input event for `Host::route`, `Host::grid`, or `Harness::send`. Positions are layout points. On the terminal host a cell is `Host::CELL` points on each side. |
| `Event` setters | `physical(PhysicalKey)`, `modifiers(ModifiersState)`, `repeat(bool)`, `text(Option<String>)`, `delta(Point)`, `kind(PointerKind)`, `button(Option<MouseButton>)`, `buttons(MouseButtons)` | Set one field of the event. A setter leaves a variant that lacks the field unchanged. |
| `events::GestureEvent`, `events::GestureKind` | `struct GestureEvent { kind, phase: ScrollPhase, position, modifiers, .. }`; `enum GestureKind { Pinch(f64), Rotate(f32), Pan(Point), DoubleTap }`; `stop_propagation()` | A trackpad or touch gesture. It goes to the node under its position and bubbles like a scroll; `Widget::on_gesture(f)` receives it. |

## Module `geometry`

| Name | Signature | Purpose |
| --- | --- | --- |
| `geometry::LogicalSize` | `struct LogicalSize { width: f32, height: f32 }` | A size in logical px or cells. |
| `geometry::PhysicalSize` | `struct PhysicalSize { width: u32, height: u32 }`; `const fn new(width: u32, height: u32) -> Self` | Size in device pixels. |


## Module `icons`

| Name | Signature | Purpose |
| --- | --- | --- |

## Node types

| Name | Signature | Purpose |
| --- | --- | --- |
| `Live`, `SortDirection` | `#[non_exhaustive] enum Live { Off, Polite, Assertive }`; `#[non_exhaustive] enum SortDirection { Ascending, Descending }` | Live-region politeness and column sort direction. `TableState` and accessibility props use the same `SortDirection`. |
| `HostMode` | `enum HostMode { Pixel, Cell }` | Tells widgets whether the host lays out in pixels (window) or in terminal cells. Default is `Pixel`. |
| `TextCursor` | `struct TextCursor { caret: Signal<usize>, anchor: Signal<Option<usize>> }` | Caret and selection anchor of an editable text node, exported to accessibility. |
| `Node` | `struct Node`; `fn text_cursor(&self) -> Option<TextCursor>` | A node in the tree, as `NodeBuilder::mutate` sees it. |
| `TextBuilder` | `struct TextBuilder`; `wrap()`, `ellipsize()`, `nowrap()`, `label(name)`, `description(text)`, `role(Role)`, `keyboard_shortcut(shortcut)`, `focusable(bool)`, `tab_index(i32)`, `hittable(bool)`, `mutate(FnOnce(&mut Node))`, `add(&mut Cx) -> NodeId` | Builder of a text node. `wrap` breaks lines at the width; `ellipsize` keeps one line and ends cut text with an ellipsis. `runs` styles ranges of the text. `label` replaces the text that screen readers read. |
| `ImageBuilder` | `alt`, `fit`, `crop`, `mutate`, plus `Widget` | The one image builder. `image` returns it. |
| `ObjectFit` | `enum ObjectFit { Fill, Contain, Cover }` | How an image fills its box. |
| `TopLayer` | `struct TopLayer` | Marker of a node that paints above the tree. |
| `Surface` | `struct Surface { key: u64 }` | Marker of a host-drawn surface. |

## Layout types

| Name | Signature | Purpose |
| --- | --- | --- |
| `Display`, `Position` | `enum Display { Block, Flex, Grid, None }`; `enum Position { Relative, Absolute }` | Set the display mode and positioning of a node. |
| `JustifyContent`, `AlignItems` | `enum JustifyContent { Start, End, Center, Stretch, SpaceBetween, SpaceAround, SpaceEvenly }`; `enum AlignItems { Start, End, Center, Baseline, Stretch }` | Set main-axis and cross-axis alignment of a node and its children. `align_self` takes `AlignItems`. |
| `Dimension` | `enum Dimension { Px(f32), Percent(f32), Auto, Fill, Rows(f32), Cols(f32), Em(f32) }` | The length of a size, padding, margin, gap, or inset. `rows`, `cols`, and `em` return it. |

| `Position` | `enum Position { Relative, Absolute }` | Positioning mode. |
| `Overflow` | `enum Overflow { Visible, Hidden, Scroll, Auto }` | Overflow mode. |
| `TrackSize` | `enum TrackSize { Px(f32), Fr(f32), Auto, MinMax(f32, f32) }` | Size of a grid track. |

## Module `profile`

| Name | Signature | Purpose |
| --- | --- | --- |
| `profile::Phase` | `enum Phase { Effect, Layout, LayoutSolve, ..., Paint, RenderCpu, A11y, TuiPaint, TuiRaster, TuiWrite, TuiPresent, Gpu, GpuWait, RenderPlan, RenderEncode, RenderRetain, RenderSubmit }` (`name()`, `category()`) | Names one timed stage of a frame. `category()` returns its `Category`. `Gpu` is GPU execution time from timestamp queries; `GpuWait` is time a headless host blocks on the GPU. The `Render*` phases split renderer CPU time into the damage plan, command encoding, the retained scene copy, and the queue submit. |
| `profile::Counter`, `profile::Cause`, `profile::BackendKind`, `profile::FallbackCause` | `#[non_exhaustive] enum` with `name() -> &'static str` | Name a per-frame counter, the event that caused a frame, the active renderer backend, and why the host fell back from the GPU backend. |
| `profile::BackendStatus` | `struct BackendStatus { kind: BackendKind, fallback: FallbackCause }`; `const TERMINAL` | Records which backend renders the frame and the fallback cause, if any. |
| `profile::span` return value | drop guard | Measures one phase from creation until drop. Create it with `profile::span`. |
| `profile::frames` element | `struct { index: u32, cause: Cause, wall_ns: u64, end_us: u64, phases: Vec<PhaseTime>, counts: Vec<CounterValue>, hot: Vec<Attribution>, gpu_passes: Vec<(&'static str, u64)>, sites: Vec<Site>, backend: Option<BackendStatus> }`; `counter`, `total_ns`, `work_ns`, `hottest` | Holds the recorded timings, counters, attributions, per-line times, and GPU pass times of one frame. `work_ns` is the frame time minus the time spent waiting for the GPU or display. |
| `profile::frames` element `sites` item | `struct { location: &'static Location, kind: SiteKind, self_ns: u64, total_ns: u64, runs: u32 }`; `SiteKind { Effect, Mount, Message }` with `name()` | The time one effect, child list, or task message handler line took in the frame, slowest first. |
| `profile::enable`, `profile::disable`, `profile::reset` | `fn enable()`; `fn disable()`; `fn reset()` | Turn frame recording on or off, and clear recorded frames. |
| `profile::begin_frame`, `profile::tick_frame` | `fn begin_frame(backend: BackendStatus)`; `fn tick_frame()` | Open a new recorded frame with its backend status, and close the current frame. Hosts call these once per frame. |
| `profile::span` | `fn span(phase: Phase) -> Span` | Starts timing `phase` in the current frame. |
| `profile::count_add` | `fn count_add(counter: Counter, n: u64)` | Add `n` to a counter in the current frame. |
| `profile::interact` | `fn interact(cause: Cause)` | Marks the next frame as caused by `cause` so the report can measure input latency. |
| `profile::dump` | `fn dump(path: impl AsRef<Path>) -> Result<(), Error>` | Write recorded frames as a Chrome trace file and print a summary. |
| `profile::frames` | `fn frames() -> Vec<Frame>` | Return the recorded frames. |

## Module `reactive`

| Name | Signature | Purpose |
| --- | --- | --- |
| `reactive::Owner` | `struct Owner`; `fn new(name: &'static str, parent: Option<Owner>) -> Self` | Reactive scope that owns effects, signals, and cleanups. Disposing it disposes its children. |
| `reactive::on_cleanup` | `fn on_cleanup(f: impl FnOnce() + 'static)` | Run `f` when the current owner is disposed. |
| `reactive::batch` | `fn batch<R>(f: impl FnOnce() -> R) -> R` | Group writes into one update. |
| `reactive::untrack` | `fn untrack<R>(f: impl FnOnce() -> R) -> R` | Run `f` without subscribing the current effect to its reads. |
| `reactive::defer` | `fn defer(f: impl FnOnce() + 'static)` | Run `f` after the current effect returns, or now outside an effect. An effect uses it to consume the signal that triggers it. |
| `Menu::events`, `Tray::events`, `media::events`, `hotkey::register` return value | `subscribe(impl Fn(T) + 'static) -> Subscription` | Receive events from the host. |

## Module `scene`

| Name | Signature | Purpose |
| --- | --- | --- |
| `scene::SceneCommand` | `enum SceneCommand { PushClip(Clip), PopClip, PushTransform(Transform), PopTransform, SaveLayer { opacity }, RestoreLayer, PushLayer(LayerScope), PopLayer, Rect(DrawRect), Gradient(DrawGradient), Pattern(DrawPattern), DrawShadow { rect, radii, shadow }, GlyphRun(GlyphRun), Sprite(DrawSprite), Image(DrawImage), Decoration(DrawDecoration) }` | One draw or state command in a `CommandScene`. Both hosts consume the same command list. |
| `scene::emit_scene` | `fn emit_scene(cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene` | Walk the laid-out tree from `root` and return its draw commands, skipping nodes outside `clip`. |
| `scene::DrawRect` | `struct DrawRect { node, rect, color, opacity, tone, leaf, outline }`; `struct CellRect { tone, leaf, border, outline }` | A filled rectangle command. `CellRect` holds the cell-grid hints (tone, leaf, border, outline) that the terminal host reads. |
| `scene::DrawRect` | `DrawRect::new(rect, color)`, `.opacity(u16)` | A filled rectangle. `SceneBuilder::draw_rect` takes it. Defaults: square corners, opaque, no stroke. |
| `scene::Transform3D` | `struct Transform3D`; `Transform3D::IDENTITY`, `.then(Transform3D)`, `.translate(x, y, z)`, `.rotate_x(radians)`, `.rotate_y(radians)`, `.rotate_z(radians)`, `.perspective(distance)`, `.around(origin: Point, inner: Transform3D)`; `From<Transform>` | A CSS 4×4 transform. Each method appends one CSS transform function, so a chain reads like a `transform` list. `around` turns `inner` about a point, like `transform-origin` and `perspective-origin`. Distances under 1 px count as 1 px. |
| `scene::LayerEffect::Projected` | `LayerEffect::Projected { transform: Transform3D, hide_backface: bool }` | Draw a layer as a flat plane through a 3D transform with perspective (CSS `transform-style: flat`). The renderer composites the layer once, as a perspective-correct quad. With `hide_backface`, a layer that shows its back draws nothing (`backface-visibility: hidden`). The part of the plane behind the viewer is clipped off and the rest draws; a layer with every corner behind the viewer draws nothing (CSS Transforms 2 §4.2). The terminal host draws the layer's projected bounds as a scale. |
| `scene::LayerEffect::Mask` | `LayerEffect::Mask(Mask)` | Scale a layer's pixels by a mask, as in CSS `mask-image`. The renderer draws the layer and the mask into two offscreen layers and composites the layer once, multiplied by the mask's alpha or luminance; it keeps the layer's opacity and rounded clip. The terminal host draws the layer unmasked. |
| `scene::LayerEffect::Filter` | `LayerEffect::Filter(Arc<[Filter]>)` | Pass a layer's pixels through each filter in order, as in CSS `filter` (Filter Effects 1 §5: the list applies in order). The renderer runs the filters on the layer's offscreen texture before it composites: each run of consecutive color filters is one pass with the spec's per-primitive clamp, a blur runs three box passes, and a drop shadow is one blur and one pass. Size the scope's clip to hold the blur and the shadow offset. The terminal host draws the layer unfiltered. |
| `scene::Filter` | `#[non_exhaustive] enum Filter { Blur(f32), DropShadow(Shadow), Brightness(f32), Contrast(f32), Grayscale(f32), HueRotate(f32), Invert(f32), Opacity(f32), Saturate(f32), Sepia(f32) }` | One CSS filter function. Color filters use the Filter Effects 1 §13 matrices on unpremultiplied sRGB (§5: filter functions operate in sRGB); `Grayscale`, `Invert`, `Opacity` and `Sepia` clamp to 1, and negative amounts clamp to 0. `Blur` and the `DropShadow` blur are standard deviations in logical pixels, blurred in premultiplied sRGB (Filter Effects 1 §5), with each GPU box pass reading at most 256 texels to a side; `DropShadow` tints the blurred alpha with the shadow color, offsets it, draws it under the layer and ignores `spread`. `HueRotate` is in degrees. |
| `scene::Mask` | `struct Mask`; `Mask::new(mode: MaskMode, scene: CommandScene)` | The pixels `scene` draws, in the coordinates of the layer's children, read as a mask. Gradients, images, rects and paths work; a layer pushed inside a mask scene is not drawn on the GPU. CSS `mask-size`, `mask-position` and `mask-repeat` resolve to the rects of these commands. |
| `scene::MaskMode` | `enum MaskMode { Alpha, Luminance }` (default `Alpha`) | Which channel of a mask scales the layer (CSS `mask-mode`). `Luminance` is the sRGB luminance (0.2125 R + 0.7154 G + 0.0721 B, as in `feColorMatrix` `luminanceToAlpha`) times alpha. |
| `scene::BlendMode` | `#[non_exhaustive] enum BlendMode { Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity }` (default `Normal`); `BlendMode::ALL` | How a layer mixes with its backdrop (CSS `mix-blend-mode`, Compositing and Blending 1 §10), in sRGB-encoded space like browsers. A layer carries it in the `blend` field of its scope. On the GPU a non-normal layer copies the backdrop region and composites once with the formula; `Normal` composites as before with no copy. A layer drawn through a 3D projection blends the same way; the terminal host ignores the mode. |
| `scene::Shadow` | `Shadow::new(offset: [f32; 2], blur: f32, spread: f32, color: [f32; 4])`, `Shadow::glow(color: Rgba8, spread, blur)` | A box shadow with linear color, offset, blur, and spread. `glow` builds a centred glow from an sRGB color. |
| `SceneBuilder::draw_text_shadow` | `draw_text_shadow(&mut self, run: &GlyphRun, shadow: Shadow)` | Draw one CSS `text-shadow` behind `run`: a copy of the run in the shadow color, offset, blurred through a `Filter::Blur` layer with `shadow.blur` as the standard deviation (CSS blur radius / 2). Draw a run's shadows last to first, then the run, so the first shadow is on top and no shadow covers the text (CSS Text Decoration 3 §4). The copy is recolored through the run color, so a color glyph (emoji) is not tinted. |

## Module `style`

| Name | Signature | Purpose |
| --- | --- | --- |
| `style::Elevation` | `enum Elevation { Sm, Base, Md, .. }` | Selects a shadow depth from the theme. |
| `style::BORDER_WIDTH` | `const BORDER_WIDTH: Px = Px(1)` | Default border width for widgets. |
| `style::space` | `const fn space(steps: u8) -> f32` | Converts a number of spacing steps to pixels. |
| `style::TextStyle` | `struct TextStyle { font_family: String, font_size: f32, font_weight: u16, italic: bool, line_height: TextLineHeight, letter_spacing_em: f32, tabular_nums: bool, decoration: TextDecoration, runs: Option<TextRuns> }` | Holds the resolved typography of a text node. `OwnedTextEngine::new` takes one. |
| `style::ThemeValue` | `enum ThemeValue { Color(Rgba8), Px(f32), Ms(u16) }` | One theme token value. |

## Module `tasks`

| Name | Signature | Purpose |
| --- | --- | --- |
| `tasks::on_message` | `fn on_message<T: Send + 'static, F: Fn(T) + 'static, C: FnOnce() + 'static>(rx: impl MessageReceiver<T>, f: F, on_close: C)` | Run `f` on the UI thread for each message, in the current owner, then `on_close` after the last message, when every sender is dropped. `rx` is a tokio `UnboundedReceiver<T>` or a bounded `Receiver<T>`. |

## Module `terminal`

| Name | Signature | Purpose |
| --- | --- | --- |
| `terminal::Caps` | `struct Caps { color, graphics: Option<Protocol>, mux: Option<Mux>, keyboard, hyperlinks, underline_styles, borders, glyphs: Glyphs, ambiguous: Ambiguous, erase: Erase, cursor, sync, clipboard, backdrop, background: Option<Rgba8>, notices: Notices, fills: Fills }`; `Caps::detect(env)`, `Caps::probe()` | Terminal capabilities. `detect` reads environment variables; `probe` also queries the terminal. When the environment leaves the color depth below truecolor, `probe` asks the terminal (DECRQSS) and trusts a kitty keyboard reply. When the environment names no terminal, as behind ssh or a multiplexer, `probe` asks for its version (XTVERSION); any answer turns hyperlinks on and a known name sets `notices`. Inside tmux or screen, only `TERM`, `COLORTERM`, and the probe identify the terminal; variables inherited from the outer terminal are ignored. `detect` picks the key protocol from the terminal name; `probe` asks the current terminal for the kitty keyboard protocol (`CSI ? u`) and uses `Legacy` when it does not answer, whatever the name. `probe` skips its other queries while typed input is waiting. When raw mode cannot be entered, `probe` returns the environment result and sends no queries. `erase` is `Erase::Default` for `TERM=screen*`, which does not erase with the current background. `SIGIL_COLOR=truecolor\|256\|16\|none` forces the color depth; `SIGIL_ASCII=1` forces `Glyphs::Ascii`; `SIGIL_AMBIGUOUS_WIDE=1` sets `Ambiguous::Wide`. |
| `terminal::caps` | `fn caps() -> Option<Caps>` | The capabilities the terminal host probed when it entered raw mode, for the current thread. `None` before the terminal host starts, after it exits, and in a window. Read `Caps::backdrop` here to pick a light or dark theme, and `Caps::background` for the color the terminal reported; `Caps::probe` called before `sigil::run` switches the terminal to raw mode for its queries and back, so replies are read, not echoed. |
| `terminal::backdrop` | `fn backdrop() -> Signal<Option<Backdrop>>` | The terminal background brightness. The terminal host sets it at startup from the OSC 11 reply, or from the OS light or dark setting when the terminal does not answer. It follows OS changes while the terminal matched the OS at startup. Read it with `get` in an effect or closure to switch themes live; with no theme file, the host already switches between `Theme::light` and `Theme::dark`. |
| `terminal::ColorDepth`, `terminal::Keyboard`, `terminal::Borders`, `terminal::Cursor` | `enum ColorDepth { Truecolor, Ansi256, Ansi16, Monochrome }`; `enum Keyboard { Legacy, Kitty }`; `enum ClipRoute { Pbcopy, Osc52 }`; `enum Borders { Rounded, Light, Ascii }`; `enum Cursor { Bar, Block }` | Capability fields of `Caps`: color depth, key protocol, clipboard route, border glyph set, and caret shape. |
| `terminal::Fills` | `enum Fills { Partial, Whole }`; `Caps::fills` | How the terminal host fills a rectangle that covers part of a cell. `Partial` draws the edge with half and eighth block glyphs; `Whole` fills whole cells only. `Caps::detect` picks `Whole` for Terminal.app, by name or by `__CFBundleIdentifier=com.apple.Terminal` behind a multiplexer, because it draws block glyphs from the font with gaps between cells. Over ssh neither variable reaches the remote side, so set `SIGIL_FILLS=whole` there; `SIGIL_FILLS=whole\|partial` overrides the detection anywhere. |
| `terminal::Backdrop` | `enum Backdrop { Dark, Light }`; `Caps::backdrop: Option<Backdrop>` | Terminal background brightness. `Caps::detect` reads it from `COLORFGBG`; `None` means unknown. With no theme file set, the terminal host uses the dark theme on `Dark`. |
| `Caps::glyphs` | `enum { Unicode, Console, Ascii }` | Glyph set of the terminal and its one glyph map. `Ascii` for a non-UTF-8 locale, `TERM=dumb`, or `TERM=vt100`; `Console` for `TERM=linux`, which keeps box drawing and full and shade blocks and maps eighth blocks and braille to the nearest supported glyph; `Unicode` otherwise. The terminal host maps every painted cell through it. `glyphs()` reads the process environment, so apps pick glyphs through the same map. |
| `terminal::notify`, `terminal::progress` | `fn notify(title: &str, body: &str) -> Result<(), Error>`; `fn progress(value: Option<u8>) -> Result<(), Error>` | Send a desktop notification, or set the tab progress (`Some(0..=100)`) and clear it (`None`). Each writes only what `Caps::notices` says the terminal supports, and nothing when no terminal host is live. |
| `terminal::Notify` | `struct Notices { pub notify: Option<Notify>, pub progress: bool }`; `enum Notify { Osc9, Osc777 }` | The notification sequence and the tab-progress sequence (OSC 9;4) a terminal supports. |
| `terminal::Mux` | `enum Mux { Tmux, Screen }` | The terminal multiplexer that `Caps` detected from `TMUX` or `STY`. Graphics output is wrapped for it. |
| `terminal::Cell` | `struct Cell { fg: Ink, bg: Ink, bold, italic, dim, lines: Lines, underline_ink: Ink, .. }`; `Cell::blank()`, `text() -> &str`, `set_text(&str)`, `link() -> Option<&str>` | One grid cell. A cell with empty text is the trailing half of a wide glyph. |
| `terminal::Ink`, `terminal::Underline` | `enum Ink { Bare, Of(Tone), Exact(Rgba8) }`; `enum Underline { None, Straight, Curly }` | Cell color (unset, theme tone, or exact RGBA) and underline shape. |
| `terminal::calibrate` | `fn calibrate(out: &mut impl Write) -> Result<(), Error>` | Set the cell width rule from how the terminal renders wide glyphs: full-screen mode measures them in the startup exchange, inline mode measures them on the cursor's row. Returns `TimedOut` without asking again when the terminal answered nothing at startup. |
| `Host::ansi` | `fn ansi(out: &mut impl Write, prev: &Grid, next: &Grid, depth: ColorDepth) -> Result<(), Error>` (feature `terminal`) | Write the escape sequences that turn the `prev` grid into the `next` grid. Profilers use it to measure the ANSI diff. |
| `Host::grid` | `fn grid(&mut self, events: impl IntoIterator<Item = Event>, cols: u16, rows: u16) -> Result<Grid, Error>` (feature `terminal`) | Run one terminal frame on a fresh `Screen` and return its cell grid. |
| `Host::paint_cells` | `fn paint_cells(&self, grid: &mut Grid, scene: &CommandScene)` (feature `terminal`) | Draw a scene into a cell grid with the host theme. |

## Module `testing`

Feature `kittest`. For tests that drive the kernel by hand; app tests use `sigil_kittest::Harness`.

| Name | Signature | Purpose |
| --- | --- | --- |
| `testing::with_cx`, `testing::with_runtime` | `fn with_cx(name: &'static str, f: impl FnOnce(&mut Cx))`; `fn with_runtime(name: &'static str, f: impl FnOnce(&Runtime))` | Run `f` inside a fresh reactive runtime, with a root `Cx` for `with_cx`. |
| `testing::tree_size` | `fn tree_size(name: &'static str, root: impl IntoNode) -> usize` | Count the nodes a tree mounts. |
| `testing::misalignments` | `fn misalignments(host: &Host, tolerance: Tolerance) -> Vec<Misalignment>` | Check column and inset alignment of the mounted tree as last laid out. Empty before `mount`. |
| `testing::emit_since`, `testing::take_delta` | `fn emit_since(host: &mut Host, viewport: (f32, f32), delta: &FrameDelta, prev: &CommandScene) -> (CommandScene, EmitDiff)`; `fn take_delta(cx: &mut Cx) -> FrameDelta` | Emit the next scene from the frame's `FrameDelta`, copying the commands of every subtree that did not change. |
| `testing::style_data` | `fn style_data(style: &LayoutStyle) -> &StyleData` | Read a style's resolved layout data. |
| `testing::tick_intervals` | `fn tick_intervals() -> usize` | Fire every due `interval` and return how many fired. A hand-driven frame loop calls it where `Host::frame` would. |
| `widgets::svg_scene`, `widgets::SvgMode` | `fn svg_scene(tree: Arc<usvg::Tree>, key: u64, size: (f32, f32), surface: Rgba8, mode: SvgMode) -> (CommandScene, Vec<String>)`; `enum SvgMode { Raster, Vector }` | The scene an SVG picture paints into a box of `size` at the origin: one resvg bitmap, or lowered scene commands with the names of the subtrees that fell back to a bitmap. Both draw in one layer that composites over any backdrop in sRGB like resvg, so `surface` is unused. A picture whose nodes, counted at each clip, mask and pattern use, pass 16,384, or whose commands pass 65,536, draws as one bitmap; pattern copies past that command budget fall back to a bitmap for the patterned shape. `key` names the tree in the image cache. For conformance runs against the resvg suite. |

## Text types

| Name | Signature | Purpose |
| --- | --- | --- |
| `app::WebFont`, `app::WebFontScript` | `struct WebFont { script: WebFontScript, url: String, face_index: u32 }`, `WebFont::new(script, url, face_index)`, `WebFont::noto_defaults()`; `enum WebFontScript { Cjk, Arabic, Hebrew, Devanagari, Khmer, Thai, Lao }` | One web fallback face and the script it covers. The Noto defaults pin exact commits on jsDelivr. |
| `app::set_web_fallbacks` | `fn set_web_fallbacks(fonts: impl IntoIterator<Item = WebFont>)` | The faces the web host fetches the first time text uses a script. Call before `sigil::run`; an empty list fetches nothing. Native targets ignore the list. |

## Module `widgets`

| Name | Signature | Purpose |
| --- | --- | --- |
| `widgets::spinner_frame` | `fn spinner_frame(tick: u64) -> &'static str` | The spinner glyph for a tick count. |
| `widgets::icon_svg` | `fn icon_svg(svg: impl Into<Arc<str>>) -> IconBuilder` | An SVG icon, drawn as vector paths. `currentColor` and a root with no `fill` take the color of the tone in the active theme, so a theme change recolors it without a raster; explicit colors in the SVG stay. |
| `widgets::TreeNode` | `struct TreeNode<K>`; `new(id, label)`, `.leading(icon)`, `.status(text, Tone)`, `.detail(text)`, `.tone(RowTone)`, `.children(iter)`, `.child(node)` | One node of a `tree`. `leading` takes an `Icon` or a text glyph. |
| `widgets::ListState`, `widgets::CursorEdge` | `struct ListState<K: Clone + Eq + Hash + 'static = String> { cursor: Signal<usize>, scroll: Signal<f32>, expanded: Signal<Vec<K>>, query: Signal<String>, caret: Signal<usize> }` (`Copy`); `new()`, `at(len) -> usize`, `shift(delta: isize, len, CursorEdge)`, `select(index)`, `is_expanded(&K) -> bool`, `toggle(&K)`, `toggle_only(&K)`, `collapse_all()`, `type_query(String)`, `filter<'a>(labels: impl IntoIterator<Item = &'a str>) -> Vec<usize>`; `enum CursorEdge { Clamp, Wrap }` | The state of a list: cursor row, scroll offset, expanded keys, and filter query with its caret. `at` clamps the cursor to `len` rows. `shift` moves it `delta` rows and stops at the ends or wraps around. `toggle_only` expands one key and collapses the rest, or collapses it when it was expanded. `filter` returns the indexes of the labels that fuzzy-match the query, best match first. |
| `widgets::disclosure` | `fn disclosure<H: IntoNode, B: IntoNode>(open: Signal<bool>, header: impl Fn(bool) -> H, body: impl Fn() -> B) -> NodeBuilder` | An expand and collapse section. The header row shows the `Disclosure` chevron and `header(open)`; a click, Enter, or Space flips `open`. The body is built only while `open` is true. |
| `widgets::ChangeKind` | `enum ChangeKind { Added, Modified, Deleted }` | Line change mark in the `code_edit` gutter. |
| `widgets::DiagSeverity`, `widgets::DiagSpan` | `enum DiagSeverity { Hint, Info, Warning, Error }`; `struct DiagSpan { line: usize, start: usize, end: usize, severity: DiagSeverity }` | Diagnostic range that `code_edit` underlines, with its severity. |
| `Rgba8::to_hex`, `Rgba8: FromStr` | `fn to_hex(self) -> String`; `"#3c59e7".parse::<Rgba8>()` | Hex text of a color. `Rgba8` parses 3, 6, or 8 hex digits, with an optional `#` or `0x`. |
| `widgets::ButtonBuilder` | `primary()`, `outline()`, `ghost()`, `soft()`, `neutral()`, `destructive()`, `icon_svg(svg)`, `activate(f)`, `size(ControlSize)`, `disabled(impl Reactive<bool>)`; `ButtonBuilder::new(label)`, `solid()`, `tone(Tone)`, `theme(Theme)`, `accessible(name)`, `pending(Signal<bool>)` | Builder of a button. Tone and fill are independent, so call order does not matter. `primary()`, `neutral()`, and `destructive()` set only the tone (`Primary`, `Neutral`, `Error`), as does `tone(Tone)`. `solid()`, `soft()`, `ghost()`, and `outline()` set only the fill: `solid()` is a filled background, `soft()` a framed tinted fill, `ghost()` no fill and no frame, `outline()` a framed ghost. With no fill set, the `Primary` and `Error` tones draw `solid()` and every other tone draws `soft()`. With no tone set, the tone is `Neutral`. `icon_svg` puts an SVG icon before the label (feature `scene`). A button below the default size, or with a height of 32 px or less, uses the `text-xs` label. `pending` disables the button and shows a spinner before the label while the signal is true. `accessible` sets the name that screen readers read. Width and height come from `Widget`. |
| `widgets::TextFieldBuilder` | `struct TextFieldBuilder`; `autofocus()`, `focus_when(Signal<u64>)`; `TextFieldBuilder::new(Signal<String>)`, `placeholder(text)`, `change(Fn(&str))`, `submit(Fn(&str))`, `blur(Fn())`, `disabled(impl Reactive<bool>)`, `invalid_when(impl Reactive<bool>)`, `password()`, `reveal(Signal<bool>)`, `clipboard(impl Clipboard)`, `caret_visible(&mut self) -> Signal<bool>`, `scroll_offset(&mut self) -> Signal<f32>` | Builder of a text input. `autofocus()` focuses the field when it mounts. `focus_when` focuses it each time the signal changes after mount. `submit` runs with the value on Enter. `blur` runs when the field loses focus. `password()` masks the text; `reveal` shows it while the signal is true. `caret_visible` and `scroll_offset` return the signals of the caret blink and the horizontal scroll. |
| `widgets::TextEditBuilder` | `struct TextEditBuilder`; `TextEditBuilder::new(Signal<String>)`, `placeholder(text)`, `change(Fn(&str))`, `submit(Fn(&str))`, `submit_on(SubmitKey)`, `disabled(impl Reactive<bool>)`, `state(TextEditState)`, `caret_position(Signal<usize>)`, `caret_visible(&mut self) -> Signal<bool>`, `chrome(bool)`, `content_padding(horizontal, vertical)`, `clipboard(impl Clipboard)` | Builder of a text editor. Its default height is 160 px in a window and 5 rows in a terminal; set another with `.height(..)`, such as `.height(rows(3.0))`. Enter inserts a new line and `submit` runs with the value on Ctrl+Enter; `submit_on(SubmitKey::Enter)` swaps them, so Enter submits and Shift+Enter inserts a new line, as a chat composer does. The placeholder shows whenever the value is empty, focused or not. `state` takes the value, caret, and undo state from a `TextEditState`. `chrome(false)` draws the editor without its frame and background. |
| `widgets::TextEditState` | `TextEditState::new(value, caret: Signal<usize>)` | Caret and undo state of a text editor. |
| `widgets::ColAlign` | `enum ColAlign { Start, End }` | Column alignment. |
| `widgets::VirtualListBuilder` | `struct VirtualListBuilder<T, R>`; `measured()`, `estimate(len)`, `scroll_bind(Signal<f32>)`, `follow_row(Signal<usize>)`, `content_height_bind(Signal<f32>)`, `max_scroll_bind(Signal<f32>)`, `cursor(Signal<usize>)`, `identified()`, `follow_end(bool)`, `initial_scroll(ScrollAnchor)`, `scroll_into_view(index)`; `row_height(len)`, `item_height(Fn(usize, &T) -> f32)`, `overscan(pages)`, `key_by(Fn(usize, &T) -> u64)`, `viewport_height(len)`, `viewport_width(px)`, `viewport_fill()`, `shrink_to_content()`, `scrollbar(bool)`, `scroll_x_bind(scroll_x, max, width)`, `keyboard(bool)`, `selection(SelectionMode)`, `selection_state(&mut self) -> Signal<SelectionState>`, `change(Fn(usize))` | Builder of a virtual list. `cursor` shares the keyboard-focused row with a signal and keeps that row in view; arrow keys write it. `identified()` keys rows by `Identifiable::id`, so selection and row state follow a row when the list changes. `follow_end(true)` opens the list at the last row and keeps it pinned there while rows are added, until the user scrolls up. `follow_row` scrolls the minimum distance to keep the row in a read-only or derived signal visible, and wins over `cursor` for scrolling; while that row is the last row, it also pins the end like `follow_end`. `initial_scroll` and `scroll_into_view` set the start position only. `measured()` lays rows out at the list width and caches their heights. The two extent binds receive the content height and the largest scroll offset. `item_height` gives each row its own height. `key_by` keys rows by the value it returns. `change` runs with the first selected index each time the selection changes. |
| `widgets::RowRender` | `trait RowRender<T> { type Row: IntoNode; fn row(&self, index: usize, item: &T) -> Self::Row; }` | Row renderer of a virtual list. Every `Fn(usize, &T) -> N` with `N: IntoNode` implements it. |
| `widgets::ScrollAnchor` | `enum ScrollAnchor { Top, Bottom, Index(usize) }` | Where a virtual list stays scrolled. |
| `widgets::SelectionState` | `SelectionState::new(mode: SelectionMode)`; `click(index)`, `ctrl_click(index)`, `shift_click(index)`; `contains(index) -> bool`, `selected_indices() -> impl Iterator<Item = usize>`, `mode() -> SelectionMode` | Selected indexes of a list. |
| `widgets::ListColumn` | `ListColumn::new(text, width)`; `start()`; `From<(TextContent, f32)>` | One fixed-width column of a list row. Columns align to the end unless `start()` is set. |
| `widgets::RowTone` | `enum RowTone { Normal, Muted, Header }` | Text tone of a list row. `Muted` dims an unavailable row. `Header` labels columns and takes no hover or selection. |
| `widgets::Highlight` | `struct Highlight { start: usize, end: usize, token: Token }` | One syntax mark and its theme token. `start` and `end` are grapheme columns. |
| `widgets::DiffBuilder` | `struct DiffBuilder`; `change(f: impl Fn(DiffView))`; `view(DiffView)`, `path(text)`, `status(text)`, `max_rows(n)`, `wrap(bool)` | Builder of a diff view. `change` shows one button per `DiffView` and calls `f` with the one the user picks. `path` shows in the header; `status` shows as a line under the rows. `wrap(true)` wraps long lines inside the row instead of ending them with `…`; a wrapped line keeps its syntax colors but not the in-line change highlight. |
| `widgets::ChartBuilder` | `bar()`, `line()`, `labels(impl IntoIterator<Item = impl Into<String>>)`, `series(name, values: impl Reactive<Vec<f32>>, tone)`; `tone(Tone)`; `horizontal()` | Builder of a chart. `horizontal` lays bars along rows with values on the x axis. `Widget::width` and `height` set its size (default 280×180, minimum 80×48). |
| `style::ControlSize` | `enum ControlSize { Compact, Default, Wide, Grow }` | Size step of a control. `Metrics` and `Theme` map it to height, width, padding, gap, and radius. |
| `widgets::SparklineBuilder` | `range(low, high)`, `tone(t)`, `tones(impl IntoIterator<Item = Option<Tone>>)`, `mark(SparkMark)`, `fill(SparkFill)`, `rows(n: usize)` | Builder of a sparkline. Without `range`, the scale runs from the lower of 0 and the minimum to the maximum. `rows(n)` draws a filled area graph `n` text rows tall, bottom-aligned. `tones` sets an optional tone per value that overrides `tone` for its column; a braille cell takes the tone of its right value, else its left. |
| `widgets::SparkMark` | `enum SparkMark { Block, Braille }` | Sparkline glyphs. `Block` (default): one column per value, 8 levels per row. `Braille`: two values per cell, 4 levels per row. |
| `widgets::SparkFill` | `enum SparkFill { Solid, Shade }` | Body of a multi-row sparkline. `Solid` (default): every cell in the tone. `Shade`: the body in a dim step of the tone and the top cell of each column solid, so a steady high value reads as a line. |
| `widgets::IconBuilder` | `tone(t)`, `size(s)`, `px(px)`, `strong()`; `IconBuilder::new(glyph)`, `label(text)` | Builder of an icon. `px` sets an exact icon size. `strong` uses the high-contrast step of the tone scale. |


## Jobs to symbols

| Job | Symbol |
| --- | --- |
| Import the application surface | `use sigil::prelude::*;` |
| Open a window and run the frame loop | `sigil::run(config, tree)` (feature `desktop`) |
| Run the tree in a terminal | `sigil::run(AppConfig::new(title).terminal(), tree)` (feature `terminal`) |
| Run the terminal host inline instead of on the alternate screen | `AppConfig::terminal().inline(true)`; `Hooks::region_rows` through `AppConfig::terminal_hooks` sizes the live region |
| Run the tree in a browser | `sigil::run(config, tree)` built for `wasm32-unknown-unknown` ([Web host](guide/web.md)) |
| `<canvas>` | `AppConfig::canvas(id)` (browser host; native hosts ignore it) |
| Open a file or URL with the operating system handler | `sigil::app::open(target)` (feature `desktop`) |
| Stop the application | `sigil::request_exit()`; in the terminal host it also works from a `terminal::Hooks` callback |
| Send a window command | `request_window_command`, `WindowCommand` (feature `desktop`) |
| Embed the tree in another engine | `Owner::scope`, `Host::new`, `Host::mount` |
| Patch the running app when the code changes | feature `hot`, `dx serve --hot-patch` ([Hot reload](guide/hot-reload.md)) |
| Route an input event to an embedded tree | `Host::route(Event)` |
| Run an accessibility action on an embedded tree | `Host::perform(NodeId, &accesskit::ActionRequest)` |
| Lay out and paint an embedded tree | `Host::layout`, then `Host::emit(viewport)` for the `CommandScene`; `Host::scene(viewport)` does both |
| Capture a terminal cell grid as a headless PNG | `Grid::image(&Theme, PhysicalSize)` returns `Result<image::RgbaImage, Error>` (features `terminal` + `scene`); `PhysicalSize` sets pixels per cell. Uses the CPU renderer and bundled monospace font, without a window or GPU. `sigil_kittest::Harness::image()` also supports cell harnesses at 10×20 pixels per cell. Terminal graphics images are rejected rather than silently omitted. |
| Draw a scene onto a cell grid | `Host::paint_cells(&mut grid, &scene)` (feature `terminal`) |
| Get the rectangles of host-drawn surfaces | `surface(key)` in the tree, then `Host::surfaces` |
| Get the accessibility tree of an embedded tree | `Host::accessibility_tree` |
| Pump external events into the terminal loop, or run code after each frame | `sigil::run(config.terminal().terminal_hooks(Hooks { .. }), tree)` |
| `$EDITOR` | `sigil::terminal::suspend(f) -> io::Result<R>` |
| Commit lines above the inline live region into scrollback | `sigil::terminal::print_above(text) -> io::Result<()>` |
| `terminal::print_above`, `NotLive`, `NotInline`, `io::ErrorKind::Unsupported`, `err.get_ref()?.downcast_ref()` | `sigil::Error` with `ErrorKind::NotLive` or `ErrorKind::NotInline` |
| Set the terminal window title | `sigil::terminal::set_title(title) -> io::Result<()>` |
| Ring the terminal bell, with an optional desktop notification | `sigil::terminal::bell(note: Option<&str>) -> io::Result<()>` |
| Send a terminal desktop notification | `sigil::terminal::notify(title, body) -> io::Result<()>` |
| Show progress in the terminal tab | `sigil::terminal::progress(Some(percent)) -> io::Result<()>` |
| Handle pasted text on a node | `.on_paste(f)` from `Widget`; `f` takes `&str` and returns true to stop the paste |
| Check whether the terminal window has focus | `sigil::terminal::focused() -> bool` |
| Check whether a native window has focus | `sigil::app::WindowId::current()` while mounting, then `.focused() -> Option<Signal<bool>>` |
| Show a desktop notification from a native window | `sigil::app::notify::Notification::new(title, body).send()` (feature `notify`) |
| Follow the OS light or dark setting | `sigil::terminal::backdrop()`, `Backdrop` |
| Send Ctrl+C to the tree instead of exiting | `AppConfig::quit_on_ctrl_c(false)` |
| Set the terminal frame interval (default 1/60 s, 60 Hz) | `AppConfig::frame_interval(Duration)` |
| `Drags`, `Off` | `sigil::terminal::set_mouse_tracking(MouseTracking) -> sigil::Result<()>` |
| Pick which terminal mouse reports the app receives | `sigil::terminal::set_mouse_tracking(tracking: MouseTracking) -> io::Result<()>` |
| Render one terminal frame to a string | `sigil_kittest::Harness::present(tree, cols, rows, events)` |
| Drive the terminal host one step at a time | `Host::new(HostMode::Cell)`, `Host::grid` |
| Read the terminal graphics protocol | `sigil::terminal::caps`, `Caps::graphics`, `sigil::terminal::Protocol` |
| Bind a key on a node | `.key(shortcut, handler)` from `Widget` |
| Bind a key that fires from anywhere | `.shortcut(shortcut, handler)` from `Widget` |
| See every key before the focused descendant | `.key_capture_any(handler)`; call `event.stop_propagation()` to keep the key from the target |
| Run a callback on a fixed period | `interval(Duration, f) -> IntervalId`, `IntervalId::cancel` |
| Run a callback once after a delay | `sigil::reactive::timeout(Duration, f) -> IntervalId` |
| Load data on demand and show loading and errors | `task(make)`, `Task::run`, `Task::cancel`, `Task::status`, `show` |
| Stream results from a backend thread | `signal_writer(signal)`, `SignalWriter::update` |
| Apply a theme | `Theme: FromStr`, `set_theme`, `watch_theme`, `AppConfig::theme_file` |
| Name a theme color in code | `sigil::style::Token` (`Token::Surface.bg()`, `Token::Ring.color()`) |
| Theme a table header or row divider | `Token::TableHeader` (`table-header`), `Token::BorderSubtle` (`border-subtle`) |

## Terminal rasterizer

The terminal host draws the same `CommandScene` as the window. Widgets do not branch on the host. The rasterizer maps each command to cells:

| Scene command | Cells |
| --- | --- |
| `Rect` | Cell background. A leaf rect fills with a full block. A rect thinner than one cell draws `─` or `│`. |
| `Rect` | Box-drawing glyphs on each side whose width is above zero. A rounded border uses rounded corners. |
| `scene::GlyphRun` | Text cells with bold and italic. `TextFlow` selects one line, wrap, or ellipsis. |
| `Image` | The detected graphics protocol, or sextant glyphs. The crop and the clip apply. |
| `PushClip`, `PushLayer` | Clip the commands inside. |
| `SaveLayer`, `PushLayer` | Blend fills, borders, and text against the cell below, with the active theme. |
| `Gradient` | Cell background in the gradient's color at its midpoint. |
| `Path`, dashed and dotted borders, gradient text | Rastered as an image, then drawn like `Image`. Gradient text draws as text in the gradient's midpoint color. |
| Shadow, pattern, sprite, decoration | Not drawn. |

TopLayer content comes last in the scene, so it draws above clipped content.

## App bundles

Package `sigil-sign`, library `sigil_sign`. Guide: [Signing](guide/signing.md).

| Name | Signature | Purpose |
| --- | --- | --- |
| `sigil_sign::AppBundle` | Fields: `name`, `identifier`, `version`, `executable`, `exe_name`, `icon`, `provisioning_profile: Option<&Path>`, `min_system` | Bundle metadata and inputs. Set `provisioning_profile` to `None` when no macOS profile is required. |
| `sigil_sign::CodeSign` | Fields: `identity: Option<&str>`, `entitlements: Option<&Path>`; `.sign(path: &Path) -> Result<(), PackError>` | Native macOS signing for an executable or a completed `.app`. `None` selects ad-hoc signing. Profiles belong to `AppBundle`. |
| `sigil_sign::macos_app` | `fn macos_app(app: &AppBundle<'_>, resources: &[ResourceCopy<'_>], dest: &Path, sign: &CodeSign<'_>) -> Result<(), PackError>` | Build the app, copy an optional regular, non-symlink profile unchanged to `Contents/embedded.provisionprofile`, then sign on macOS. A profile requires a nonempty identity other than `-`. |
| `sigil_sign::Dmg` | Fields: `app: &Path`, `volume: &str`, `background: Option<&Path>`, `icon: Option<&Path>`, `layout: Option<&Path>` | macOS only. Disk image inputs. `layout` is a `.DS_Store` file for the disk root. |
| `sigil_sign::macos_dmg` | `fn macos_dmg(dmg: &Dmg<'_>, dest: &Path, sign: &CodeSign<'_>) -> Result<(), PackError>` | macOS only. Build a compressed image with the app and an `Applications` link, then sign it. |
| `sigil_sign::notarize` | `fn notarize(file: &Path, profile: &str) -> Result<(), PackError>` | macOS only. Submit to Apple's notary service with a `notarytool` keychain profile, wait, and staple the ticket. |
| `sigil_sign::windows_dir` | `fn windows_dir(app: &AppBundle<'_>, resources: &[ResourceCopy<'_>], dest: &Path) -> Result<(), PackError>` | Build the Windows app directory. Rejects `AppBundle::provisioning_profile`. |

`sigil-pack macos --provisioning-profile PATH` sets the profile. Other commands reject this flag. `--identity` or `SIGIL_CODESIGN_IDENTITY` supplies the identity. Profile embedding does not validate the profile's contents, expiration, application identifier, or entitlements.

## Errors

| Type | Returned by | Meaning |
| --- | --- | --- |
| `sigil::Error` | Every fallible call in the crate | One error type. `Error::kind` returns an `ErrorKind`; branch on it, never on the message. `Error::new` wraps any `std::error::Error` as `Other`, `Error::with_kind` sets the kind, `Error::message` builds one from a kind and a message, and `Error::node` returns the missing node of a `NodeMissing` error. `source()` returns the cause. |
| `sigil_sign::SignError` | `sign_file`, `verify_file` | `KeyFile`, `Key`, `Package`, `Envelope`, `TrustedComment`, or `PasswordRequired`: the key, the package, the signature, or the signed file name or version failed, or an encrypted key had no password. |
| `sigil_sign::PackError` | `macos_app`, `windows_dir` | `Missing`, `Io`, `Codesign`, `Tool`, `Escape`, `Symlink`, `Occupied`, or `NotFile`. |

## Test harness

Package `sigil-kittest`, library `sigil_kittest`. Guide: [Testing](guide/testing.md).

| Name | Signature | Purpose |
| --- | --- | --- |
| `sigil_kittest::Harness` | `Harness::new(app: impl IntoNode + 'static) -> Harness`; `.size(cols: u16, rows: u16)`; `.pixels(width: u32, height: u32)`; `.scale(device_scale: f32)`; `.gpu(width: u32, height: u32, scale: f32)` | Mounts a tree on a headless host with a task runtime installed. `size` uses the terminal cell host (default 80×24); `pixels` uses the window host and renders offscreen through wgpu, at the logical size times `scale` (default 1). `gpu` renders on a GPU surface, and its `image()` reads the frame back. Each frame applies finished task results. |
| `Harness::gpu` | `fn gpu(self, width: u32, height: u32, scale: f32) -> Self` | Paint every frame through the GPU renderer into an offscreen texture at `scale` and wait for the GPU, so profiles include emit, submit, and GPU time. Without a GPU adapter it falls back to `pixels`. Every harness frame is a profile frame. |
| `Harness::send`, `Harness::settle`, `Harness::advance` | `fn send(&mut self, events: impl IntoIterator<Item = Event>) -> &mut Self`; `fn settle(&mut self) -> &mut Self`; `fn advance(&mut self, by: Duration) -> &mut Self` | Route events through `Host::frame` and run frames until no signal write or animation is pending (at most `SETTLE_SAFETY_CAP` frames). `advance` moves the virtual interval clock and runs 1/60 s frames across `by`. |
| `Harness::click`, `hover`, `scroll`, `drag` | `fn click(&mut self, target: &(impl Target + ?Sized)) -> &mut Self`; `fn scroll(&mut self, target: &(impl Target + ?Sized), lines: f32)`; `fn drag(&mut self, from: &(impl Target + ?Sized), to: &(impl Target + ?Sized))` | Pointer actions at the centre of a labelled control, a `Node`, or a `Point` (layout points). A label waits up to `EXPECT_TIMEOUT` for its control. Each action settles. |
| `Harness::press`, `press_with`, `type_text`, `type_into`, `focus_next`, `focus_previous` | `fn press(&mut self, key: impl Into<LogicalKey>) -> &mut Self`; `fn press_with(&mut self, key, modifiers: ModifiersState)`; `fn type_text(&mut self, text: &str)`; `fn type_into(&mut self, label: &str, text: &str)` | Key down and up, one key per character, or Tab and Shift+Tab. `type_into` clicks the control with that label, then types. Each action settles. |
| `Harness::expect_text` | `fn expect_text(&mut self, text: &str) -> &mut Self` | Wait until a node's name or value is `text`, running frames that apply finished tasks, up to `EXPECT_TIMEOUT`; then fail with the tree dump. |
| `Harness::get`, `get_by_role`, `get_by_text`, `query`, `find_all`, `focused`, `refresh` | `fn get_by_role(&mut self, role: Role, name: &str) -> Node`; `fn get_by_text(&mut self, text: &str) -> Node`; `fn find_all(&mut self, query: &Query) -> Vec<Node>`; `fn refresh(&mut self, node: &Node) -> Option<Node>` | Query the accessibility tree. `get*` fails the test with the tree dump unless exactly one node matches. Text runs and hidden nodes are skipped. |
| `Harness::flaws`, `assert_no_flaws`, `unaudited` | `fn flaws(&mut self) -> Vec<layout::audit::Flaw>`; `fn assert_no_flaws(&mut self)`; `fn unaudited(self) -> Self` | Run the design rules with the tolerance for the harness host. `assert_no_flaws` panics with every finding and the `text()` dump. A harness that was mounted runs it when dropped, unless the test panicked or called `unaudited`. |
| `Harness::text`, `accessibility_snapshot`, `accessibility_tree`, `image`, `host` | `fn text(&mut self) -> String`; `fn image(&mut self) -> Rendered`; `fn host(&mut self) -> &mut Host` | The terminal grid text (the accessibility dump on a pixel harness), the tree, the offscreen image, or the host for anything else. |
| `Harness::snapshot`, `snapshot_with` | `fn snapshot(&mut self, name: &str)`; `fn snapshot_with(&mut self, name: &str, max_delta_pct: f32)` | Compare with `tests/goldens/<name>.txt` (cells) or `<name>.png` (pixels). `SIGIL_REGEN_GOLDENS=1` rewrites them. |
| `sigil_kittest::Node` | `role()`, `name()`, `value()`, `numeric()`, `bounds()`, `center()`, `is_focused()`, `is_checked()`, `is_selected()`, `is_expanded()`, `is_disabled()` | A snapshot of one accessibility node at query time. The name follows `labelled_by` when the node has no label. |
| `sigil_kittest::Query` | `Query::role(role)`, `Query::text(text)`, `.name(name)` | Match by role, accessible name, or text (name or value). |
| `sigil_kittest::Target` | `trait Target { fn point(&self, ui: &mut Harness) -> Point }` for `str`, `Node`, and `Point` | Where a pointer action lands. A `str` is a label: the one control whose name or value it is, else the one node. A label on two controls fails. |
| `sigil_kittest::EXPECT_TIMEOUT` | `const EXPECT_TIMEOUT: Duration` (5 s) | How long `expect_text` and a label target wait. |
| `Harness::render`, `sigil_kittest::Render` | `fn render<B: IntoNode>(build: B) -> Render<B>`; setters `.mode(mode)`, `.size([w, h])`, `.scale(scale)`, `.hit((x, y))`, `.measure(|| ...)`, `.stub()`, `.focus_first()`; `.image() -> Rendered`; `.capture(&mut settle) -> Result<Option<(RgbaImage, Option<Rect>)>, Error>`; `.filmstrip(frames, dt) -> Rendered` | Render one widget offscreen in an interaction state. `capture` also returns the top layer bounds and runs layout again while `settle` returns true. `filmstrip` puts hover frames side by side. |
| `Harness::present` | `fn present<N: IntoNode>(tree: impl Fn() -> N + 'static, cols: u16, rows: u16, events: &[Event]) -> Result<String, sigil::Error>` | Run one terminal frame of a tree and return the screen text. |
| `sigil_kittest::Golden` | `Golden::new(stem)`; `.delta(pct)`; `.path()`; `.assert(rendered)`; `.assert_image(&image)`; `.assert_differs(&other)` | A reference image at `tests/goldens/<stem>.png` under `CARGO_MANIFEST_DIR`. `SIGIL_REGEN_GOLDENS` rebakes it. |
| `sigil_kittest::FakeMeasure` | `FakeMeasure::fixed(width, height)`; `FakeMeasure::chars(char_width, line_height)`; `Default` is `fixed(0.0, 0.0)` | The test `TextMeasure`: one box per text node, or the widest line times `char_width` and the line count times `line_height`. Carets advance 10 px per byte on 16 px lines. |

## Inspection

With the `inspect` cargo feature, an app exposes a local control socket when `SIGIL_INSPECT` is set. `SIGIL_INSPECT=1` binds `127.0.0.1:5761`; any other value is an address to bind. The socket speaks one JSON object per line, one reply per request.

| `op` | Request | Reply |
| --- | --- | --- |
| `a11y::tree` | – | `nodes`: id to `{role, name, description, bounds, children, focused, selected, expanded, toggled, value, numeric}` plus `root` and `focus`. Bounds are in points on the window host and cells on the terminal host. |
| `a11y::snapshot` | – | `text`: the accessibility text dump. |
| `screenshot` | – | Terminal: `text` with the painted grid. Window: `png` (base64) plus `width`/`height` in physical pixels. |
| `layout::audit` | – | `flaws`: a list of `{kind, node, detail}`, where `kind` is a `FlawKind` name and `detail` is the `Flaw` display text. |
| `surfaces` | – | `surfaces`: keyed surface holes and rects. |
| `settle` | – | `ok` after servicing; `settled` is false while the frame stays dirty or animations run. |
| `click` | `col`/`row` (terminal cells) or `x`/`y` (points), `button` | Moves, presses, and releases the pointer through the normal router. |
| `move`, `down`, `up` | same coordinates | One pointer phase. |
| `widgets::scroll` | coordinates plus `dx`/`dy` in lines | A scroll event. |
| `key` | `key` (a `NamedKey` name or one character), `ctrl`/`shift`/`alt`/`meta` | Key down and up. |
| `type` | `text` | Key events per character. |
| `profile` | `last` (frame count, default 30) | `enabled`, `frames` (index, cause, `wall_ms`, per-phase ms, counters, `backend`), and `summary` (the exit report text). |
| `profile_dump` | `path` (optional) | Writes the Chrome trace JSON now and replies with `path`. Defaults to `SIGIL_PROFILE`'s value or `sigil-profile.json`. |
| `profile_on`, `profile_off` | – | Enable or stop frame collection without restarting. |
| `hud` | `on` (default true) | Draws the frame-HUD overlay: a sparkline, the active backend with any fallback cause, and last-frame phase readout top-right. Terminal host draws cells; the desktop host overlays scene commands at paint. Enabling turns profiling on. |

Requests are serviced between frames on the UI thread, so replies reflect the tree as last laid out. The socket is loopback by convention; the feature is off without the environment variable.

## Conventions

- Make reactive writes on the thread that owns the runtime. From
  another thread, use `signal_writer`, or `on_message` for a channel.
- `sigil::run` installs the OS clipboard on the window host. The
  terminal host installs an OSC 52 clipboard; on a local macOS terminal it writes with `pbcopy` and reads with `pbpaste`. Over OSC 52 the clipboard cannot be read, so a text input leaves Ctrl+V to the app and the terminal's own paste arrives as a paste event. An embed host calls `install(OsClipboard)` itself.
- Public event, command, error, and config types are
  `#[non_exhaustive]`. Match them with a `_` arm. Build `AppConfig` with its builder methods.
- `SIGIL_BACKEND` accepts `auto`, `wgpu`, and `cpu`. See
  [Troubleshooting](guide/troubleshooting.md).
- `sigil::profile` keys are typed. `span(Phase)` times a phase and
  `count_add(Counter, n)` records counters. `name()` and `Display` give the trace text. `Frame::counter(Counter)` reads a counter; `Frame::phases` and `Frame::counts` hold `PhaseTime` and `CounterValue` pairs.
- `profile::begin_frame(BackendStatus)` opens a frame for one host. A
  `BackendStatus` is a `BackendKind` (`Gpu`, `GpuSoftware`, `Cpu`, `Terminal`) and a `FallbackCause` (`None`, `NoAdapter`, `DeviceError`, `ForcedByEnv`, `UnknownEnv`). `BackendStatus::TERMINAL` is the terminal host's status. `Frame::backend` holds the status of the host that drew that frame, so a window host and a terminal host in one process keep separate statuses.
- `profile::interact(Cause)` marks the input that caused a frame.
  `Cause` is `Idle`, `Pointer`, `Hover`, `Key`, `Scroll`, `Ime`, `Resize`, `A11y`, `Expose`, or `Input`. When several inputs land in one frame, the highest `Cause::rank()` wins. `Frame::cause` holds it.
- Parsers for markdown, HTML, SVG, and theme files run on the UI
  thread. See `SECURITY.md` for the input surfaces.
