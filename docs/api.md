# Sigil API map

This page lists every item that `sigil` exports at the crate root and
in `sigil::prelude`. Each item has a signature, a one-line purpose, and
an example. Every example on this page compiles as a doctest of the
`sigil` crate.

The package is `sigil-ui`. The library is `sigil`. The API can change
between 0.x releases.

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

The default feature `desktop` gives the window host. The feature
`terminal` gives the terminal host. A terminal-only program uses
`default-features = false, features = ["terminal"]`. The README lists
every Cargo feature. For task-oriented examples, see
[Recipes](guide/recipes.md).

## Crate root

Modules are public so that you can name an item by its path, for
example `sigil::style::Token`. Prefer the prelude and the functions in
this table.

| Name | Signature | Purpose |
| --- | --- | --- |
| `geometry` | `mod geometry` | Points, rectangles, and sizes in logical and physical pixels, and the device scale factor. |
| `prelude` | `mod prelude` | The application import surface. |
| `a11y` | `mod a11y` | Accessibility tree construction. |
| `animation` | `mod animation` | Easing, springs, and keyframes. |
| `animation_runtime` | `mod animation_runtime` | Animators that drive signals each frame. |
| `app` | `mod app` | Application host, configuration, and theme files. |
| `bidi` | `mod bidi` | Bidirectional text: paragraph levels, visual order, and `ParagraphBidi::visual_step` for caret motion. `text::bidi` re-exports it. |
| `clipboard` | `mod clipboard` | Clipboard trait and the installed clipboard. |
| `events` | `mod events` | Event routing, handlers, and focus. |
| `hal` | `mod hal` | Input events, keys, and geometry types. |
| `icons` | `mod icons` | Bundled icon glyphs (feature `scene`). |
| `kernel` | `mod kernel` | Node tree, builders, and roles. |
| `layout` | `mod layout` | Layout styles and the layout engine. |
| `profile` | `mod profile` | Frame timing counters. |
| `reactive` | `mod reactive` | Signals, effects, owners, and intervals. |
| `renderer` | `mod renderer` | The GPU and CPU renderers (feature `scene`). |
| `scene` | `mod scene` | The `CommandScene` display list. |
| `shell` | `mod shell` | The window shell (feature `desktop`). |
| `style` | `mod style` | Paint, themes, tokens, and metrics. |
| `tasks` | `mod tasks` | Async tasks and cross-thread dispatch. |
| `terminal` | `mod terminal` | The terminal host (feature `terminal`). |
| `text` | `mod text` | Fonts and text shaping (feature `scene`). |
| `widgets` | `mod widgets` | Widget builders. |
| `wgpu` | `extern crate wgpu` | The `wgpu` version that the renderer uses (feature `scene`). |
| `run` | `fn run<C: Into<AppConfig>, N: IntoNode>(config: C, app: impl Fn() -> N + 'static) -> AppResult` | Run the app component on the host that `config` selects. `app` is called once, inside the root's reactive owner, and again after a hot patch. |
| `request_exit` | `fn request_exit()` | Ask the running host to exit after this frame. |
| `dialog::FileDialog` | `FileDialog::new()`; `filter(name, &[ext])`, `directory(path)`, `file_name(name)`; `open_file()`, `open_files()`, `pick_folder()`, `save_file()` | Builder of a file dialog. Each pick returns a `Picked` signal that turns from `Pending` to `Ok(None)` on cancel or `Ok(Some(path))`. The in-app picker returns one path from `open_files()`. |
| `dialog::Picked` | `type Picked<T> = Signal<Status<Option<T>, Infallible>>` | Result of a file dialog. |
| `dialog::open_file`, `dialog::save_file`, `dialog::pick_folder` | `fn() -> Picked<PathBuf>` | `FileDialog::new()` with one pick. |
| `dialog::message`, `dialog::confirm` | `fn(title, body) -> Signal<Status<bool, Infallible>>` | An alert with OK, or OK and Cancel. Resolves `Ok(true)` on OK. |
| `dialog::mock` | `push_path`, `push_paths`, `push_alert`, `reset` (`cfg(test)` or feature `kittest`) | Queues the next dialog answers. A queued answer resolves the signal at once and opens no dialog. |
| `AppConfig` | `struct AppConfig`; `From<&str>`, `From<String>`; `AppConfig::new(title)`; host: `window()` (feature `desktop`), `terminal()` (feature `terminal`), `canvas(id)` (feature `desktop`); window: `size(w, h)`, `window_size() -> (u32, u32)`, and with feature `desktop` `min_size(w, h)`, `user_zoom(f32)`, `frameless(bool)`, `overlay(bool)`, `anchor(Anchor)`; `theme_file(path)`, `panic_policy(PanicPolicy)`, `quit_on_ctrl_c(bool)`, `frame_interval(Duration)` | Run configuration: title, size, host, theme file, web canvas. A title converts to a config with the defaults; `AppConfig::new(title)` starts the builder. `window()` selects the window host and `terminal()` the terminal host. Without a call, the host is the window when feature `desktop` is on, else the terminal. A host method exists only with its feature, so a build without that host does not compile the call. `canvas` names the web canvas element. `user_zoom` scales the UI; the default is 1.0. `frameless(true)` removes the window frame. `overlay(true)` opens a transparent accessory window; `anchor` places it on the screen. `theme_file` loads the theme from a file and reloads it when the file changes. `quit_on_ctrl_c` is true by default. `frame_interval` sets the wait between terminal host frames; the default is 33 ms. |
| `AppResult` | `type AppResult<T> = Result<T, Error>` | Result of the application functions. |
| `Error` | `struct Error` | Opaque application error. |
| `Host` | `struct Host` | Embed host: build, route, lay out, and paint one tree. `set_caps(HostCaps)` tells widgets what the host can show. |

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
    let _: sigil::AppResult<()> = Err(sigil::Error::with_kind(sigil::app::ErrorKind::Io, std::fmt::Error));
    let _ = sigil::app::PanicPolicy::Isolate;
    let _ = config.clone().frame_interval(Duration::from_millis(33));
    if let Some(theme) = sigil::app::parse_theme("base = \"dark\"\n[tokens]\naccent = \"#3c59e7\"\n") {
        sigil::app::set_theme(theme);
    }
    let _: fn(std::path::PathBuf) -> sigil::AppResult<sigil::app::ThemeWatcher> = sigil::app::watch_theme;
    let _ = sigil::app::open("https://example.com");
    let mut host = sigil::Host::new();
    host.mount(app_tree);
    let _: Option<sigil::app::AccessibilityTree> = None;
    let _ = sigil::app::WindowCommand::DragResize(sigil::app::ResizeDirection::East);
    let _ = sigil::app::WindowOpts::default();
    let _ = sigil::app::Anchor::BottomCenter { margin: 16.0 };
    sigil::app::set_default_fonts(sigil::app::DefaultFonts::default());
    sigil::app::use_system_fonts();
    let tuning: sigil::app::TextTuning = sigil::app::text_tuning();
    sigil::app::set_text_tuning(tuning);
    let _ = sigil::widgets::interaction::InteractionState::new(true);
    let _ = sigil::style::Token::Accent;
    let _ = sigil::scene::CommandScene::default();
    let _: Option<sigil::wgpu::TextureFormat> = None;
    let _ = sigil::terminal::focused();
    let _ = (sigil::layout::cols(2.0), sigil::reactive::batch(|| 1));
    let _ = sigil::animation::Easing::Linear;
    let _ = sigil::animation_runtime::MotionPrefs::reduced();
    let _ = sigil::tasks::ScheduleError::Closed;
    let _ = sigil::widgets::spacer();
    let _ = sigil::events::FocusIntent::Down;
    let _ = sigil::events::KeyPhase::Down;
    let _ = sigil::kernel::container();
    let _ = sigil::clipboard::paste();
    let _ = sigil::app::set_theme;
    sigil::run(config, app_tree)
}
# }
# fn main() {}
```

The modules `bidi`, `icons`, `profile`, `renderer`, `shell`, and
`text` have no stable entry point in 0.1. Use them only through the
functions above.

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
fn menu_events() -> sigil::reactive::EventChannel<sigil::app::menu::MenuId> {
    sigil::app::menu::events()
}

#[cfg(feature = "tray")]
fn tray_events() -> sigil::reactive::EventChannel<tray_icon::TrayIconEvent> {
    sigil::app::tray::events()
}

#[cfg(feature = "render")]
use sigil::terminal::render as _;

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
use sigil::Error;
use sigil::app::is_grid;
use sigil::app::parse_theme;
use sigil::app::viewport_size;
use sigil::prelude::*;
use sigil::Host;
use sigil::reactive::Runtime;

fn tree() -> impl IntoNode {
    let size = viewport_size();
    text(derived(move || format!("{:?} grid={}", size.get(), is_grid())))
}

fn main() -> AppResult<()> {
    let _: Option<(Host, ThemeWatcher, Error)> = None;
    let _ = Runtime::scope("demo", || text("x"));
    set_theme(parse_theme("base = \"light\"\n[tokens]\n").unwrap_or_else(Theme::light));
    let _ = (open("https://example.com"), watch_theme("theme.toml").map(drop));
    run(AppConfig::new("Demo"), tree)
}

#[cfg(feature = "audio")]
fn player() -> sigil::app::Player { sigil::app::Player::new() }
#[cfg(feature = "pty")]
fn session() -> AppResult<sigil::app::Session> { sigil::app::Session::spawn("sh") }
```

### Nodes and text

| Name | Signature | Purpose |
| --- | --- | --- |
| `kernel::Cx` | `struct Cx`; `cx.add(node) -> NodeId` | The node tree. App code does not name it; a component is a function that returns `impl IntoNode`. Embedders and `on_added` hooks use it. |
| `kernel::NodeId` | `struct NodeId` | Handle of a node in the tree. |
| `NodeBuilder` | `struct NodeBuilder`; `child(impl IntoNode)`, `children_reactive(Fn() -> IntoIterator<Item = N>)`, `children_keyed(Fn() -> IntoIterator<Item = (K, N)>)`, `with<T>(value)`, `mutate(FnOnce(&mut Node))`, `add(&mut Cx) -> NodeId`; accessibility: `role(Role)`, `label(name)`, `description(text)`, `placeholder(text)`, `labelled_by(NodeId)`, `owns(NodeId)`, `action(Action)`, `keyboard_shortcut(shortcut)`, `focusable(bool)`, `tab_index(i32)`, `traps_tab(bool)`, `hittable(bool)`, `checked(Signal<bool>)`, `selected(Signal<bool>)`, `expanded(Signal<bool>)`, `value(Signal<f32>, min, max)`, `value_step(f32)`, `text_value(Signal<String>)`, `text_cursor(caret, anchor)`, `row_count(Signal<usize>)`, `row_index(usize)`, `column_count(usize)`, `column_index(usize)`, `sort_direction(Signal<Option<SortDirection>>)`, `on_added`; `From` every widget builder | Builder that `container` and most widgets return. `NodeBuilder::from(text(..))` or `.into()` puts any widget builder into a `Vec<NodeBuilder>`. `children_reactive` builds the children again each time a signal that the closure reads changes. `children_keyed` reconciles the children by key, like `for_each`. `with` attaches a typed value to the node (`NodeExtensions`). `hittable(false)` lets pointer events pass through the node. The accessibility methods set what screen readers get for the node. |
| `IntoNode` | `trait IntoNode` | A value that builds into a node. |
| `node` | `fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode>` | Box a child of any builder type. |
| `container` | `fn container() -> NodeBuilder` | A box that holds children. |
| `fragment` | `fn fragment() -> NodeBuilder` | Children without a box of their own. |
| `show` | `fn show<A: IntoNode, B: IntoNode>(cond: impl Reactive<bool>, then: impl Fn() -> A, otherwise: impl Fn() -> B) -> NodeBuilder` | Mounts `then()` while `cond` is true and `otherwise()` while it is false. `cond` is a `Signal<bool>` or a closure. The mounted branch keeps its node until `cond` flips. |
| `for_each` | `fn for_each<T, K: Clone + Eq + Hash, N: IntoNode>(items: Signal<Vec<T>>, key: impl Fn(&T) -> K, render: impl Fn(&T) -> N) -> NodeBuilder` | One child per item, reconciled by key. A row whose key stays keeps its node and state; new keys mount, missing keys unmount, and the order follows `items`. `render` runs for every item on each change, and only the rows of new keys mount. A row reads signals to update in place. A row helper `fn row(item: &T) -> impl IntoNode` captures the borrow under edition 2024; return a concrete builder such as `NodeBuilder`, or write `-> impl IntoNode + use<>`. |
| `switch` | `fn switch<K: Clone + PartialEq>(key: impl Reactive<K>) -> SwitchBuilder<K>`; `.case(key: K, view: impl Fn() -> N)`, `.otherwise(view: impl Fn() -> N)`, `.keep_alive(bool)` | Mounts the view of the case whose key equals `key`, or `otherwise` when no case matches. Without `keep_alive`, only the active case is mounted and the others unmount. With `keep_alive(true)`, a case mounts the first time it is active, and after that it is hidden while inactive, so it keeps its state. `show` is the two-way form. |
| `match_status` | `fn match_status<T, E: From<String>>(status: impl Reactive<Status<T, E>>) -> StatusView<T, E>`; `.idle(view)`, `.pending(view)`, `.ok(impl Fn(&T) -> N)`, `.error(impl Fn(&E) -> N)` | Shows one view for each `Status` variant. A view that is not set shows nothing. `Panicked(msg)` goes to the error view as `E::from("task panicked: {msg}")`. Each status change mounts the view again. |
| `text` | `fn text(source: impl Into<TextContent>) -> TextBuilder` | The one text node. `source` is a `&str` or `String` (static), a `Signal<T: Display>` (live), or a closure `Fn() -> String` (recomputed when the signals it reads change). `.nowrap()` keeps it on one line. |
| `TextContent` | `enum TextContent`; `From<&str>`, `From<String>`, `From<Signal<T: Display>>`, `From<F: Fn() -> String>` | Static, signal, or computed text. Every text-taking builder accepts it. |
| `image` | `fn image(source: ImageSource) -> sigil::kernel::ImageBuilder` | An image node from an `ImageSource`. |
| `ImageSource` | `ImageSource::from_rgba8(w, h, rgba) -> Option<ImageSource>` | Decoded RGBA pixels. |
| `ImageRaster::at` | `fn at(&self, width: u32, height: u32, scale: f32) -> Option<ImageSource>` | Pixels for a physical size in device pixels at `scale`. `None` when the source cannot draw at that size. |
| `Role` | `accesskit::Role` | Accessibility role. |
| `Action` | `accesskit::Action` | Accessibility action. |

```rust,no_run
use sigil::kernel::A11yProps;
use sigil::kernel::AccessibleName;
use sigil::kernel::ImageCrop;
use sigil::kernel::Node;
use sigil::kernel::NodeExtensions;
use sigil::kernel::NodeType;
use sigil::kernel::ObjectFit;
use sigil::kernel::TextBuilder;
use sigil::kernel::TopLayer;
use sigil::prelude::*;
use sigil::kernel::Surface;

fn tree() -> impl IntoNode {
    let pixels = ImageSource::from_rgba8(1, 1, vec![255, 0, 0, 255]);
    let picture = pixels.map(image).map(|i| i.fit(ObjectFit::Contain));
    let heading: TextBuilder = text("Title").role(Role::Heading);
    let body = container().label(AccessibleName::Static("Body".into())).action(Action::Click);
    let marked = container().mutate(|n: &mut Node| drop((&n.kind as &NodeType, &n.a11y as &A11yProps)));
    let items = signal(vec![1_u32, 2, 3]);
    let open = signal(true);
    let rows = for_each(items, |n: &u32| *n, |n: &u32| text(n.to_string()));
    let panel = show(open, || text("Open"), || text("Closed"));
    let children: Vec<Box<dyn IntoNode>> = vec![node(heading), node(text(|| "live".into())), node(rows), node(panel)];
    let _: Option<(ImageCrop, Surface, TopLayer, &NodeExtensions)> = None;
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
| `row` | `fn row() -> NodeBuilder` | A container that places its children left to right with the theme gap: 8 px in a window, one cell in a terminal. |
| `column` | `fn column() -> NodeBuilder` | A container that places its children top to bottom with the theme gap. |
| `LayoutStyle` | `LayoutStyle::flex_column()`, `flex_row()`, `grid(cols)`, `block()`; `LayoutStyle::pane(w, h)`, `with_layout(FnOnce(&mut StyleData))`, `data() -> &StyleData`; size: `width(d)`, `height(d)`, `fixed_size(w, h)`, `fill()`, `fill_width()`, `fill_height()`, `min_width(d)`, `min_height(d)`, `min_size(w, h)`, `max_width(d)`, `max_height(d)`, `max_size(w, h)`, `box_sizing(BoxSizing)`; flex: `flex_direction(FlexDirection)`, `flex_wrap(FlexWrap)`, `flex_basis(d)`, `grow(f32)`, `shrink(f32)`; align: `align_items(AlignItems)`, `align_self(AlignSelf)`, `align_content(AlignContent)`, `justify_content(JustifyContent)`; grid: `grid_columns(Vec<TrackSize>)`, `grid_rows(Vec<TrackSize>)`, `grid_area(col, row)`, `grid_column_span(n)`, `grid_row_span(n)`, `grid_dense()`; position: `display(Display)`, `position(Position)`, `inset(v)`, `inset_top/right/bottom/left(v)`; `overflow(Overflow)`, `overflow_x(Overflow)`, `overflow_y(Overflow)` | Layout description of a node. `fill_width`, `fill_height`, and `fill` take the full size of the parent on that axis. `grow` also sets an `auto` minimum size to 0, so the node can shrink below its content. `pane(w, h)` is `flex_column().fixed_size(w, h).gap(4.0)`. `grid_area` sets the start column and row. |
| `LayoutStyle` spacing | `padding(v)`, `padding_x/y(v)`, `padding_top/right/bottom/left(v)`, `margin(v)`, `margin_x(v)`, `margin_y(v)`, `margin_top(v)`, `margin_right(v)`, `margin_bottom(v)`, `margin_left(v)`, `gap(v)`, `gap_x(v)`, `gap_y(v)` | Padding, margin, and gap in px or text units. `margin_*` take the same type as `margin`; `gap_x` and `gap_y` take the same type as `gap`. |
| `LayoutBuilderExt` | `fn layout(self, style: LayoutStyle) -> Self` | Set the layout of a node or text. |
| `WidgetLayoutExt` | `fn layout`, `width`, `height`, `grow`, `map_node(FnOnce(NodeBuilder) -> NodeBuilder)` | Override the layout of a widget builder or an image. `layout` sets only the fields its `LayoutStyle` sets; the widget keeps the rest. `map_node` queues a change to the root `NodeBuilder` of the widget, for example `.map_node(\|n\| n.on_added(f))`. The widget applies queued changes once, after its own build. |
| `LayoutReactiveExt` | `fn position_reactive`, `size_reactive`, `bounds_reactive` | Write the laid-out position or size to a signal. |
| `Display` | `enum Display { Block, Flex, Grid, None }` | Display mode. |
| `FlexWrap` | `enum FlexWrap { NoWrap, Wrap, WrapReverse }` | Wrap mode of a flex box. |
| `AlignItems` | `enum AlignItems { Start, End, Center, Baseline, Stretch }` | Cross-axis alignment of children. |
| `JustifyContent` | `enum JustifyContent` | Main-axis distribution of children. |
| `rows` | `const fn rows(count: f32) -> TextLength` | Height of `count` text lines. |
| `cols` | `const fn cols(count: f32) -> TextLength` | Width of `count` character cells. |
| `em` | `const fn em(count: f32) -> TextLength` | `count` times the font size. |
| `IntoPx` | `trait IntoPx { fn into_px(self) -> f32; }` | A pixel count or a `TextLength`, for widget sizes such as `width` and `row_height`. |

```rust,no_run
use sigil::layout::AlignSelf;
use sigil::layout::BoundsReactive;
use sigil::layout::FlexDirection;
use sigil::layout::LayoutEngine;
use sigil::layout::LayoutResult;
use sigil::layout::LengthPercentageAuto;
use sigil::layout::Overflow;
use sigil::layout::Position;
use sigil::layout::ReactivePosition;
use sigil::layout::TextLength;
use sigil::layout::TrackSize;
use sigil::prelude::*;
use sigil::layout::LayoutError;
use sigil::layout::TextMeasure;

fn tree() -> impl IntoNode {
    let size = signal((0.0_f32, 0.0_f32));
    let grid = LayoutStyle::grid(vec![TrackSize::Px(120.0), TrackSize::Fr(1.0), TrackSize::Auto]);
    let column = LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)).min_width(em(20.0));
    let row = LayoutStyle::flex_row().flex_wrap(FlexWrap::Wrap).align_items(AlignItems::Center);
    let row = row.justify_content(JustifyContent::SpaceBetween).align_self(AlignSelf::Stretch);
    let abs = LayoutStyle::block().position(Position::Absolute).inset(LengthPercentageAuto::Px(0.0));
    let abs = abs.overflow(Overflow::Hidden).display(Display::Flex).flex_direction(FlexDirection::Row);
    let _ = (sigil::layout::Dimension::from(rows(2.0)), sigil::layout::LengthPercentage::from(cols(1.0)), TextLength::Em(1.0));
    let _: Option<(&LayoutEngine, LayoutResult, LayoutError, ReactivePosition, BoundsReactive)> = None;
    let _: Option<&dyn TextMeasure> = None;
    let button = button("OK").width(80.0).grow(1.0);
    container()
        .layout(column)
        .child(container().layout(grid).size_reactive(size))
        .child(container().layout(row).child(button))
        .child(container().layout(abs))
}

fn main() {
    let _ = tree;
    fn _ext<T: WidgetLayoutExt + LayoutBuilderExt + LayoutReactiveExt>(_: T) {}
}
```

A row or column is as wide as its content. When a child fills, with a
`Fill` width or a `grow` factor in a row, the container takes the width
that its parent offers, and a row shares that width among its children.
A `text_field` fills, so a row with a field and a button keeps the
button on screen.

See [Units](guide/units.md) for the rules that map `rows`, `cols`, and
`em` to pixels and cells.

### State

| Name | Signature | Purpose |
| --- | --- | --- |
| `Signal` | `struct Signal<T>`; `get`, `peek`, `set`, `update`, `with`; `PartialEq`, `Eq`, `Hash` by identity | Reactive value. `set` skips a value equal to the current one; `update` always notifies. Two handles are equal when they name the same signal, so a store of signals can itself live in a signal. |
| `signal` | `fn signal<T: 'static>(initial: T) -> Signal<T>` | Create a signal. |
| `derived` | `fn derived<T: Clone + PartialEq, F: FnMut() -> T>(f: F) -> Signal<T>` | A signal that `f` recomputes. Notifies readers only when the new value differs from the old one. |
| `effect` | `fn effect<F: FnMut() + 'static>(f: F)` | Run `f` now and after each change it reads. |
| `Reactive` | `trait Reactive<T>`; `fn read(&self) -> T`; `fn into_signal(self) -> Signal<T>` | A value source: a `Signal<T>`, a closure `Fn() -> T`, or a plain value (`bool`, `String`, `&'static str` as `Reactive<String>`, `Vec<T>` or `[T; N]` as `Reactive<Vec<T>>`). Show/hide, `invalid_when`, and the content widgets `diff`, `mermaid`, and `sparkline` take one. `disabled` on every widget takes one, so `.disabled(move \|\| name.get().is_empty())` needs no `derived`. |
| `interval` | `fn interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId` | Run `callback` on a fixed period. |
| `IntervalId` | `struct IntervalId` | Handle for `sigil::reactive::cancel_interval`. |
| `reactive::__test_set_clock`, `__test_advance_clock`, `__test_clear_clock` | `fn(Instant)`, `fn(Duration)`, `fn()` | Drive the interval clock by hand in tests (feature `test-clock`). |

```rust,no_run
use sigil::reactive::EventChannel;
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
    let channel: EventChannel<String> = EventChannel::new();
    drop(channel.subscribe(|msg| println!("{msg}")));
    let id: IntervalId = interval(Duration::from_secs(1), move || count.update(|n| *n += 1));
    sigil::reactive::cancel_interval(id);
    channel.emit(source.read().to_string());
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
| `task_with` | `fn task_with<A, T: Send, E: Send, F: Fn(A) -> Fut, Fut: Future<Output = Result<T, E>> + Send>(make: F) -> Task<T, E, A>` | A task whose runs take an argument: `run_with(arg)` calls `make(arg)`, so one task serves many targets, such as staging one path. |
| `Task` | `struct Task<T, E, A = ()>` (Copy); `run()` when `A` is `()`, `run_with(arg: A)`, `cancel()`, `status() -> Status<T, E>` | `run` sets `Pending`, aborts the previous run, and starts a new one; a result from a replaced run is dropped. `cancel` aborts the run in flight and sets `Pending` back to `Idle`; a finished status stays, and a result that arrives after `cancel` is dropped. `status` reads and subscribes; it needs `T: Clone` and `E: Clone`. The owner's cleanup aborts a pending run. |
| `spawn` | `fn spawn<T, E, F: Future<Output = Result<T, E>> + Send>(future: F) -> Signal<Status<T, E>>` | Start a future now, off the UI thread, and write its result to the signal. Runs once and cannot be cancelled; the owner's cleanup aborts it. For work that reruns or cancels, use `task`. For a future that owns state it cannot rebuild, such as a channel receiver. |
| `tasks::sleep` | `async fn sleep(duration: Duration)` | Wait inside a spawned future without blocking a thread. Uses the tokio timer on native and `setTimeout` on the web. |
| `tasks::blocking` | `async fn blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(f: F) -> T` | Run blocking work, such as a subprocess or a file read, inside a `task` or `spawn` future without stalling a runtime worker. Native: runs `f` on the tokio blocking pool. Web: runs `f` inline. A panic in `f` makes the task `Panicked`. Cancelling the task drops the result; `f` still runs to the end. |
| `tasks::TaskWorkers` | `#[non_exhaustive] enum TaskWorkers { Auto, Fixed(NonZeroUsize) }`; default `Auto` | Worker threads of the native task runtime. `Auto` uses one per CPU. |
| `tasks::AsyncRuntime::with_workers` | `fn with_workers(workers: TaskWorkers) -> io::Result<AsyncRuntime>` | Like `AsyncRuntime::new`, with a worker count. The native runtime starts on the first spawn, so an app that never spawns owns no worker threads. |
| `tasks::TaskHandle` | `abort()`, `is_finished() -> bool` | A task from `Scheduler::spawn` on `AsyncRuntime`. The owner's cleanup aborts it. Native tasks run on tokio; web tasks run on `wasm_bindgen_futures::spawn_local`. |
| `Status` | `enum Status<T, E> { Idle, Pending, Ok(T), Err(E), Panicked(String) }`; `is_pending()`, `is_ok()`, `is_err()`, `is_panicked()` | State of a task. A `Task` is `Idle` until its first run; `spawn` starts `Pending`. |
| `signal_writer` | `fn signal_writer<T: Send>(signal: Signal<T>) -> SignalWriter<T>` | A `Send` handle that writes a signal. Panics outside a reactive scope, or when no runtime is installed with `AsyncRuntime::install_current`. |
| `SignalWriter` | `set(value) -> Result<(), ScheduleError>`, `update(f) -> Result<(), ScheduleError>`; `Clone` | Write a signal from another thread. Clone one writer into each `task` run to stream tokens into a signal; no channel is needed. |
| `ScheduleError` | `enum ScheduleError { Closed, Full }` | A cross-thread write was not queued. |

```rust,no_run
use sigil::tasks::on_message;
use sigil::tasks::on_message_or_close;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let log = signal(String::new());
    let writer: SignalWriter<String> = signal_writer(log);
    std::thread::spawn(move || {
        let _: Result<(), ScheduleError> = writer.update(|s| s.push_str("line\n"));
    });
    let fetched: Signal<Status<u32, String>> = spawn(async { Ok::<u32, String>(42) });
    let reload: Task<u32, String> = task(|| async { Ok(7) });
    reload.run();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    on_message(rx, move |line| log.update(|s| s.push_str(&line)));
    let (done_tx, done_rx) = tokio::sync::mpsc::channel::<String>(16);
    on_message_or_close(done_rx, move |line| log.set(line), || println!("worker closed"));
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
| `NodeEventsExt` | `key`, `key_any`, `key_capture`, `key_capture_any`, `shortcut`, `pointer_down`, `on_scroll`, `on_gesture`, `on_paste`, `focus_in`, `link`, `on_selection_copy`, `autofocus`, `focus_when(Signal<u64>)`, ... | Attach event handlers to a `NodeBuilder` or to any widget builder that implements `WidgetLayoutExt`. On a widget builder the handler goes on the root node of the widget, after the handlers of the widget. `autofocus()` gives the node keyboard focus when it mounts, and again each time a focus scope that holds it, such as a modal, opens; `focus_when(trigger)` focuses it each time the signal changes, with no `NodeId` plumbing. `link(f)` receives the URL when a link inside the node, such as one in `markdown`, is clicked; the deepest handler on the path gets it. `on_selection_copy(f)` receives the text and the clipboard result each time a `selectable_text` region inside the node copies a selection. |
| `request_focus` | `fn request_focus(id: NodeId)` | Move keyboard focus to a node. |
| `Shortcut` | `Shortcut::new(key, mods)`, `plain(key)`, `ctrl(key)`, `primary(key)`, `label()`, `label_for(mac: bool)`; `FromStr` (`"ctrl+shift+k".parse::<Shortcut>()`) | A key plus modifiers. `parse` reads the names `label()` prints, in either form (`⇧⌘K` or `Shift+Super+K`), plus `cmd`, `ctrl`, `alt`, `option`; an unknown name is a `ShortcutParseError`. `label()` prints it for the running platform: `⇧⌘K` on macOS, `Shift+Super+K` elsewhere; `label_for` picks the style explicitly. `primary` uses `events::PRIMARY`: Command on macOS, Control elsewhere, for example `Shortcut::primary(LogicalKey::character("s"))` for Cmd+S or Ctrl+S. |
| `events::PRIMARY` | `const PRIMARY: ModifiersState` | The platform's primary shortcut modifier: Command (`META`) on macOS, Control elsewhere, including wasm32. |
| `Key` | `keyboard_types::NamedKey` | A named key such as `Key::Enter`. |
| `LogicalKey` | `LogicalKey::named(key)`, `LogicalKey::character(s)` | A named key or a character. |
| `ModifiersState` | `keyboard_types::Modifiers` | Modifier flags such as `CONTROL`. |
| `KeyEvent` | `struct KeyEvent { key, physical, modifiers, repeat, phase, text }` | A key press or release. |
| `KeyPhase` | `enum KeyPhase { Down, Up }` | Press or release. |
| `PointerEvent` | `struct PointerEvent` | A pointer event. `capture_pointer()` sends the following moves and the release to the node that asked, and only to it: they do not bubble to its ancestors. `position` is relative to the handler's node; `window_position` is in window coordinates. `pointer_id` tells touch contacts apart (0 for the mouse); `pressure` and `tilt` are `Some` for touch and pen input that reports them. |
| `PointerKind` | `enum PointerKind { Mouse, Touch, Pen }` | Pointer device. |
| `PointerPhase` | `enum PointerPhase { Down, Move, Up, Enter, Leave, Cancel }` | Pointer event phase. |
| `MouseButton` | `enum MouseButton { Left, Right, Middle, Back, Forward, Other(u16) }` | One mouse button. |
| `MouseButtons` | `struct MouseButtons`; `MouseButtons::LEFT` | Set of pressed buttons. |
| `Point` | `struct Point { x: f32, y: f32 }` | A position in logical px. |
| `Size` | `struct Size { width: Dim, height: Dim }`; `Size::max_content()`, `Size::min_content()` | A size constraint. The two constructors ask for the max-content or min-content size on both axes. |

```rust,no_run
use sigil::app::ResizeDirection;
use sigil::app::WindowCommand;
use sigil::events::EventHandlers;
use sigil::events::FileDropEvent;
use sigil::events::FileDropPhase;
use sigil::events::FocusIntent;
use sigil::events::ImeEvent;
use sigil::events::PhysicalKey;
use sigil::events::ScrollDelta;
use sigil::events::ScrollEvent;
use sigil::events::ScrollPhase;
use sigil::events::WindowEvent;
use sigil::events::request_window_command;
use sigil::geometry::Dim;
use sigil::geometry::LogicalSize;
use sigil::prelude::*;
use sigil::events::EventRouter;
use sigil::events::PointerEventInit;
use sigil::events::ShellEvent;

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
        .file_drop(|event: &FileDropEvent| drop(event))
        .on_ime(|event: &ImeEvent| drop(matches!(event, ImeEvent::Commit(_))))
        .on_added(|_, id| request_focus(id));
    request_window_command(WindowCommand::DragResize(ResizeDirection::South));
    root
}

fn main() {
    let _ = tree;
    let _ = (PointerKind::Mouse, PointerPhase::Down, MouseButton::Left, MouseButtons::LEFT);
    let _ = (ScrollDelta::Lines(Point { x: 0.0, y: 1.0 }), ScrollPhase::Moved, FileDropPhase::Dropped);
    let _ = (FocusIntent::Down, PhysicalKey::KeyA, Size { width: Dim::MinContent, height: Dim::Fixed(1.0) });
    let _ = LogicalSize { width: 80.0, height: 24.0 };
    let _: Option<(ShellEvent, WindowEvent, PointerEventInit, EventHandlers)> = None;
    let _ = EventRouter::new();
}
```

### Style

| Name | Signature | Purpose |
| --- | --- | --- |
| `NodeStyleExt` | `theme`, `tone`, `focus_ring`, `presence(Signal<bool>, AnimationConfig)`, `animate`, `visible_when(impl Reactive<bool>)`, `paint_when(impl Reactive<bool>, \|s\| ..)` | Theme, tone, focus ring, presence, and animation of a `NodeBuilder`. `visible_when` keeps the node mounted and hides it while the source is false; `show` mounts and unmounts instead. `paint_when` applies the paint from its closure while the source is true. `presence` animates the node in and out. |
| `Paint` | `bg(c)`, `background_gradient(g)`, `text_color(c)`, `border_color(c)`, `border_w(px)`, `border_top/right/bottom/left(px)`, `rounded(Radius)`, `shadow(Elevation)`, `opacity(f32)`, `font_size(f32)`, `font_weight(u16)`, `font_italic(bool)`, `font_family(FontFamily)`, `line_height_px(px)`, `letter_spacing_em(em)`, `tabular_nums(bool)`, `underline()`, `line_through()`, `transition(id)`, `duration(ms)` | Typed paint on nodes, text, widget builders, and state closures. `c` is `impl Into<ColorOrToken>`. Sealed. |
| `StateVariants` | `fn rest(f)`, `hover(f)`, `focus(f)`, `pressed(f)`, `disabled_style(f)`; `f: FnOnce(StateStyleBuilder) -> StateStyleBuilder` | Paint per interaction state. Works on a plain node, a text node, and a widget. Sealed. |
| `Token` | `enum Token { Accent, Surface, OnSurface, Border, .. }` | A theme colour. Pass it to `bg`, `text_color`, or `border_color`. |
| `Radius` | `enum Radius { None, Sm, Base, Md, Lg, Full }` | Corner radius from the theme: 2, 4, 6, 8, and 9999 px. 0 on a character grid. |
| `Elevation` | `enum Elevation { Sm, Base, Md, Lg, Xl }` | Shadow depth from the theme. |
| `ControlSize` | `enum ControlSize { Compact, Default, Wide, Grow }` | Size step of a control, as in `button(..).size(ControlSize::Compact)`. |
| `FontFamily` | `enum FontFamily { Sans, Mono, Serif, Icon, Display }` | Font family for `font_family`. |
| `Rgba8` | `struct Rgba8 { r, g, b, a: u8 }` | An 8-bit RGBA colour. |
| `ColorAtom` | `enum ColorAtom { Rgba8(Rgba8), ThemeRef(ColorKey) }` | A colour or a theme colour key. |
| `Token::scale` / `Token::scale_step` | `fn scale(ScaleRole, u8) -> Option<Token>`, `fn scale_step(self) -> Option<(ScaleRole, u8)>` | Map a scale role and step 1..=12 to its token and back. |
| `Theme` | `Theme::light()`, `Theme::dark()`, `Theme::console_dark()` | Theme values. |
| `Theme::resolve_color` | `fn resolve_color(&self, key: ColorKey) -> Rgba8` | Resolve a colour key. Order: runtime override, theme table, built-in value. An unknown `Custom` name panics in debug builds and paints `missing_token_fallback` with one `tracing` warning per name in release builds. |
| `Theme::color` | `fn color(&self, token: Token) -> Rgba8` | `resolve_color` for a built-in token. |
| `Theme::register_token` / `with_token` | `fn register_token(&mut self, name: &str, value: ThemeValue) -> Option<ThemeValue>` | Set a runtime override. A built-in token name also updates the matching base colour field. |
| `Theme::with_theme_tokens` | `fn with_theme_tokens(self, tokens: &phf::Map<&'static str, ThemeValue>) -> Self` | Fill the theme table layer from static data. |
| `Tone` | `enum Tone { Neutral, Primary, Info, Success, Warn, Error }`; `token(step: u8) -> Token`, `shade()`, `fill()`, `solid()` | Semantic colour family. `token` returns the tone's colour-scale step (1–12); `shade` is step 4 (tinted background), `fill` step 9 (solid fill), `solid` step 11 (text and strokes). |
| `FocusRing` | `FocusRing::static_visible(bool)`; `theme_color(Token)` | Focus ring settings. `theme_color` sets the ring colour from a theme token. |

Paint calls write to one store per node. A later call wins for each
property that it sets, whether it is a `Paint` method, a state variant,
or `.layout`. A background, border, or size that you set on a widget
holds in every state, over the widget's own defaults. To keep a
widget's hover and press feedback in another colour, set `.tone(..)`
instead of a colour. On a widget builder, the `Paint` methods win over
`width`, `height`, `grow`, and `layout` for the same property.

```rust,no_run
use sigil::style::ColorOrToken;
use sigil::style::NumericAtomKind;
use sigil::style::StyleState;
use sigil::style::ThemeValue;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let card = container()
        .layout(LayoutStyle::default().padding(8.0).margin_x(16.0).gap_y(4.0))
        .bg(Rgba8 { r: 20, g: 20, b: 30, a: 255 })
        .rounded(Radius::Base)
        .shadow(Elevation::Md)
        .border_w(1)
        .border_color(Token::Border)
        .hover(|s| s.bg(Token::SurfaceHover))
        .disabled_style(|s| s.opacity(0.5))
        .child(text("Title").font_size(14.0).font_weight(600).text_color(Token::OnSurface))
        .child(text("42").font_family(FontFamily::Mono).tabular_nums(true));
    let _ = (ColorOrToken::from(Token::Accent), Token::Accent.color(), ThemeValue::Ms(150));
    let _ = (StyleState { disabled: false, hovered: true, pressed: false, focused: false }, NumericAtomKind::Opacity);
    let theme = Theme::dark();
    let node = container().tone(Tone::Primary).focus_ring(FocusRing::static_visible(true));
    fragment().child(node.theme(theme)).child(card)
}

fn main() {
    let _ = tree;
    fn _traits<T: Paint + StateVariants>(_: T) {}
    fn _node<T: NodeStyleExt>(_: T) {}
}
```

### Animation

| Name | Signature | Purpose |
| --- | --- | --- |

```rust,no_run
use sigil::animation::AnimationDirection;
use sigil::animation::AnimationFill;
use sigil::animation::CubicBezier;
use sigil::animation::Easing;
use sigil::animation::Keyframe;
use sigil::animation::Keyframes;
use sigil::animation::SpringPreset;
use sigil::animation_runtime::AnimationConfig;
use sigil::animation_runtime::Animator;
use sigil::animation_runtime::MotionPrefs;
use sigil::animation_runtime::Playback;
use sigil::style::NumericAtomKind;
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let opacity = signal(0.0_f32);
    let _ = Animator::new(opacity, AnimationConfig::Spring { preset: SpringPreset::Snappy });
    let easing = Easing::Bezier(CubicBezier::new(0.4, 0.0, 0.2, 1.0));
    let _ = AnimationConfig::Tween { ms: 200, easing };
    let frames = Keyframes::new([Keyframe::new(0.0, 0.0), Keyframe::new(1.0, 1.0)], Duration::from_millis(300));
    let _ = (AnimationDirection::Alternate, AnimationFill::Forwards, MotionPrefs::reduced());
    let _ = Playback::new(opacity, frames);
    container().animate(NumericAtomKind::Opacity, opacity)
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
| `checkbox` | `fn checkbox(checked: Signal<bool>) -> CheckboxBuilder` | A checkbox. |
| `toggle` | `const fn toggle(on: Signal<bool>) -> ToggleBuilder` | An on/off switch. |
| `radio_group` | `fn radio_group(selected: Signal<String>) -> RadioGroupBuilder` | A set of radio options; `.label()` and `.labelled_by()` name the group. |
| `segmented` | `fn segmented(active: Signal<usize>, labels: Vec<String>) -> SegmentedBuilder` | A framed row of options where one is active; `.label()` names the group and `.height()` (`WidgetLayoutExt`) sets its height. |
| `slider` | `fn slider(value: Signal<f32>, range: RangeInclusive<f32>) -> SliderBuilder` | A value slider. |
| `Orientation` | `enum Orientation { Horizontal, Vertical }` | Axis of a slider or splitter. |
| `text_field` | `fn text_field(value: Signal<String>) -> TextFieldBuilder`; `.leading(node)`, `.trailing(node)`, `.clearable()` | A single-line text input. `leading` and `trailing` place a node, such as an icon, inside the field border; the input fills the rest. `clearable` shows a clear button while the value is not empty; it empties the value, fires `change`, and puts focus back on the input. |
| `text_edit` | `fn text_edit(value: Signal<String>) -> TextEditBuilder`; `.autofocus()`, `.focus_when(Signal<u64>)`, `.caret_rect(Signal<Option<Rect>>)` | A multi-line text editor. `autofocus` focuses it when it mounts; `focus_when` focuses it each time the signal changes. `caret_rect` receives the caret rectangle, relative to the editor's box, each time the caret moves, and `None` while the editor is disabled, so a popup can follow the caret. |
| `number_field` | `fn number_field<T: NumericKind>(value: Signal<T>, range: RangeInclusive<T>) -> NumberFieldBuilder<T>` | A numeric input. Text that parses updates `value` as you type; on Enter or blur, text that does not parse is replaced by the stored value, so the field never shows a number it did not store. |
| `NumericKind` | `trait NumericKind` | Number types that `number_field` accepts. |
| `date_field` | `fn date_field(value: Signal<Date>) -> DateFieldBuilder` | A date input. |
| `Date` | `struct Date { year: i32, month: u8, day: u8 }` | A calendar date. |
| `color_field` | `fn color_field(value: Signal<Rgba8>) -> ColorFieldBuilder` | A colour input. |
| `field` | `fn field<C: IntoNode>(label, control: C) -> FieldBuilder<C>`; `.error(impl Reactive<Option<String>>)` when `C: FieldControl`; `.required(impl Reactive<bool>)` | A label above a control. `.required` marks the control required for screen readers and shows a marker after the label while it is true. `.error` shows the message under the control and paints the control invalid while it is `Some`. The control is marked invalid while an error shows and is described by the message. |
| `chip` | `fn chip(label, tone: Tone) -> ChipBuilder`; `.dot()`, `.leading(node)` | A small tag. `dot` puts a dot in the chip's tone before the label, for status pills; `leading` puts any node, such as an icon, before it. |
| `interactive` | `fn interactive(inner: NodeBuilder) -> InteractiveBuilder` | Make any node hoverable, pressable, and focusable. |

```rust,no_run
use sigil::kernel::TextBuilder;
use sigil::widgets::ButtonBuilder;
use sigil::widgets::CheckboxBuilder;
use sigil::widgets::ChipBuilder;
use sigil::widgets::DateFieldBuilder;
use sigil::widgets::InteractiveBuilder;
use sigil::widgets::NumberFieldBuilder;
use sigil::widgets::RadioGroupBuilder;
use sigil::widgets::SliderBuilder;
use sigil::widgets::TextEditBuilder;
use sigil::widgets::TextEditState;
use sigil::widgets::TextFieldBuilder;
use sigil::widgets::ToggleBuilder;
use sigil::widgets::interaction::InteractionState;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let name = signal(String::new());
    let on = signal(false);
    let save: ButtonBuilder = button("Save").primary().activate(|| println!("saved"));
    let close: ButtonBuilder = icon_button("x", || sigil::request_exit());
    let _: (CheckboxBuilder, ToggleBuilder) = (checkbox(on), toggle(on));
    let _: RadioGroupBuilder = radio_group(signal("a".to_string()));
    let _: SliderBuilder = slider(signal(0.5), 0.0..=1.0);
    let _ = Orientation::Vertical;
    let _: TextEditBuilder = text_edit(signal(String::new()));
    let _ = TextEditState::new(name, signal(0));
    let _: NumberFieldBuilder<i32> = number_field(signal(3_i32), 0..=10);
    let _: DateFieldBuilder = date_field(signal(Date { year: 2026, month: 1, day: 1 }));
    let _ = (color_field(signal(Rgba8 { r: 0, g: 0, b: 0, a: 255 })), field("Name", text_field(name)));
    let checked = field("Name", text_field(name)).error(move || name.with(|n| n.is_empty().then(|| "Required".to_string())));
    let _: ChipBuilder = chip("beta", Tone::Info);
    let _: (ChipBuilder, TextBuilder) = (chip("3", Tone::Error), text("Label").nowrap());
    let _: InteractiveBuilder = interactive(container());
    let _ = InteractionState::new(true);
    let _: Option<TextFieldBuilder> = None;
    fn _numeric<T: NumericKind>() {}
    container().child(checked).child(save).child(close)
}

fn main() {
    let _ = tree;
}
```

### Widgets: choice, collections, and documents

| Name | Signature | Purpose |
| --- | --- | --- |
| `dropdown` | `fn dropdown(selected: Signal<usize>, options: Vec<String>) -> DropdownBuilder` | A select box. The trigger fills the width of its parent, like a form field; in a toolbar row give it `.width(..)`. |
| `combobox` | `fn combobox(state: Combobox) -> NodeBuilder` | A text input with a filtered list. |
| `Combobox` | `Combobox::new(options: Vec<String>)`; `selected_option() -> Option<String>`, `edit_query(text: &str)`, `move_cursor(delta: i32)` | State of a combobox. |
| `menu` | `fn menu(open: Signal<bool>) -> MenuBuilder` | A popup menu. |
| `context_menu` | `fn context_menu(target, items: impl Fn() -> Vec<MenuItem>) -> NodeBuilder` | Wraps `target`; a right click opens a menu of `items()` at the pointer. |
| `MenuItem` | `struct MenuItem { label, shortcut, disabled, submenu_indicator }`; `MenuItem::new(label)`, `shortcut(hint)`, `disabled(bool)`, `submenu(bool)`, `activate(Fn())` | One menu entry. `activate` runs when the entry is pressed. |
| `menu_bar` | `fn menu_bar() -> MenuBarBuilder`; `.fill()` | An in-window menu bar, as wide as its menus. `fill` makes it take the full width. |
| `command_menu` | `fn command_menu() -> CommandMenuBuilder` | A command palette. Typing in its query field filters and ranks items with `fuzzy_score`; Up and Down move, Enter or a click runs an item or opens a submenu, and Escape goes back one level. |
| `fuzzy_score` | `fn fuzzy_score(query: &str, text: &str) -> Option<FuzzyMatch>` | Rank text against a query. A lower `score` is a better match. |
| `table` | `fn table<Row>(state: TableState<Row>) -> TableBuilder<Row>` | A sortable, filterable table. |
| `TableState` | `TableState::new(rows: Vec<Row>, columns: Vec<TableColumn<Row>>)`; `TableState::identified(rows, columns)` for `Row: Identifiable`; `replace_rows(impl Into<Arc<Vec<Row>>>)`, `row_eq(self, impl Fn(&Row, &Row) -> bool) -> Self`, `selected_row() -> Option<Row>`, `clear_selection()`, `show_column(id: &'static str)`, `reset_columns()`; sort: `sort(id, SortDir)`, `toggle_sort(id)`, `cycle_sort()`, `reverse_sort()`, `sort_key() -> Signal<Option<SortKey>>`; search: `quick_filter() -> Signal<String>`; view: `visible() -> Signal<Rc<Vec<u32>>>`, `rows() -> Signal<Arc<Vec<Row>>>`; selection: `selection() -> Signal<Rc<HashSet<u32>>>`, `selected_index() -> Option<u32>`, `select_only(row)`, `select_by(delta: i32)`, `toggle_selected(row)`, `select_range_to(row)`; columns: `columns()`, `visible_columns()`, `hide_column(id)`, `show_all_columns()`, `move_column(from, to)`, `column_layout() -> Signal<Rc<ColumnLayout>>`, `column_width(id) -> Option<f32>`, `pin_column_width(id, px)`, `clear_column_width(id)` | Rows, columns, sort, and selection. `identified` keeps the selection on the same row ids across `replace_rows`. `reset_columns` restores the original column order and shows every column. `row_eq` tells the table when a replaced row is unchanged, so its cells are not rebuilt; without it every replace rebuilds the mounted rows. `toggle_sort` steps a sortable column through ascending, descending, and no sort; set `sort_key` to sort directly. `cycle_sort` moves the sort to the next sortable column in column order, ascending, wrapping to the first; with no sort it picks the first sortable column. When the sort or `quick_filter` hides a selected row, the row leaves the selection, so `selected_row` only returns a visible row. `reverse_sort` flips the direction of the current sort and does nothing without one. `select_by` moves the selection `delta` rows in visible order from the last selected row, clamped to the visible rows, and selects only that row; with no selected row, a negative `delta` selects the last visible row and any other the first. Text in `quick_filter` keeps the rows that a `filterable` column matches. `visible` holds the row indexes after filter and sort. Row numbers in the selection methods are indexes into `rows`. `select_range_to` selects from the last selected row to `row` in visible order. `pin_column_width` clamps to `MIN_COL_WIDTH`. |
| `TableColumn` | `TableColumn::new(id: &'static str, title)`; `sort_by(Fn(&Row) -> K)` with `K: Ord`, `sortable(Fn(&Row, &Row) -> Ordering)`, `filterable(Fn(&Row, &str) -> bool)`, `cell(Fn(&Row) -> N)` with `N: IntoNode`, `numeric()`, `align(ColAlign)`, `width(ColWidth)`, `min_width(px)` | One table column. `sort_by` and `sortable` make the header sort the rows; `filterable` makes the column match the text in `TableState::quick_filter`; `cell` renders the cell, else the table shows nothing for it. `numeric` right-aligns the column and uses tabular digits. |
| `ColWidth` | `enum ColWidth { Fixed(f32), Fraction(f32) }` | Column width. |
| `Density` | `enum Density { Grid, Compact, Default }` | Row density of a table. |
| `Identifiable` | `trait Identifiable { type Id; fn id(&self) -> Self::Id; }` | Stable row identity. |
| `virtual_list` | `fn virtual_list<T, R: Fn(usize, &T) -> N, N: IntoNode, I: IntoList<T, R>>(items: I, render: R) -> VirtualListBuilder<T, R>` | A list that builds only the visible rows. `render` returns any `IntoNode`, such as `text(..)` or a widget builder. |
| `SelectionMode` | `enum SelectionMode { None, Single, Multi, Extend }` | Selection mode of a list. |
| `list_row` | `fn list_row(selected: bool, title, detail) -> NodeBuilder` | One row of a list. |
| `list_row_with` | `fn list_row_with(selected: bool, title, slots: ListRowSlots) -> NodeBuilder` | A list row with a marker, status, and columns. With `slots.activate` set, the row takes keyboard focus and runs it on Enter, Space, or a click. |
| `ListRowSlots` | `struct ListRowSlots { depth, disclosure, marker, marker_tone, leading, status, status_tone, badge, columns, shrink, title_width, indent, tone, activate: Option<Rc<dyn Fn()>> }` | Optional parts of a list row. `title_width` gives the title a fixed width so columns line up after it. `leading` places a node, such as an icon, before the title. `marker_tone` and `status_tone` colour the marker and status text per row. `badge: Option<(TextContent, Tone)>` draws a count pill after the status. `indent` sets the columns per depth level (default 2). |
| `Disclosure` | `enum Disclosure { None, Leaf, Collapsed, Expanded }` | The tree chevron slot of a list row. |
| `kv_grid` | `fn kv_grid(rows: impl IntoIterator<Item = (impl Into<TextContent>, impl Into<TextContent>)>) -> NodeBuilder` | A two-column key/value grid. Keys and values can be reactive. |
| `kv_row` | `fn kv_row(label, value: impl IntoNode + 'static) -> NodeBuilder` | One key/value row. |
| `tabs` | `fn tabs() -> TabsBuilder`; `.tab(label, component)`, `.active(Signal<usize>)` | Tabs. Each `tab` adds a label and its content component; the component builds once, so a tab keeps its state while another is shown. When tabs have content, the active tab's content fills the space left below the tab strip. |
| `code_edit` | `fn code_edit(lines: Signal<Vec<String>>, caret: Signal<(usize, usize)>) -> CodeEditBuilder` | A code editor with built-in `text_edit` editing. |
| `highlight` | `fn highlight(language: &str, source: &str) -> Vec<Vec<Highlight>>` | Syntax marks per line for rust, sh, toml, json, python, javascript, typescript, go, c/cpp, yaml, css, html, sql, markdown, and diff. Other languages get no marks. `language_for_path(path)` names the language of a file path; `text_runs(marks, range)` turns marks into `TextRun`s for a range of a line. |
| `hl_spans` | `fn hl_spans(language: &str, source: &str) -> Vec<Vec<HlSpan>>` | `highlight` as column spans for `code_edit(..).highlights(..)`. |
| `markdown` | `fn markdown(source: Signal<String>) -> NodeBuilder` | Rendered markdown. Each link is exposed as a `Link` node with its URL and on-screen area, and a click on it reaches the nearest `link` handler. |
| `markdown_inline` | `fn markdown_inline(source: &str) -> TextBuilder` | One wrapped paragraph with inline markdown spans. |
| `markdown_edit` | `fn markdown_edit(source: Signal<String>) -> MarkdownEditBuilder` | A markdown editor with preview. |
| `html` | `fn html(source: Signal<String>) -> NodeBuilder` | Rendered HTML subset. |
| `mermaid` | `fn mermaid(source: impl Reactive<String>) -> NodeBuilder` | A rendered mermaid diagram. A signal or closure source draws the diagram again when it changes. |
| `diff` | `fn diff(patch: impl DiffSource + 'static) -> DiffBuilder`; `.select(Signal<Selection>)` | A rendered unified diff of text or a parsed `Patch`. A signal or closure source renders the rows again when it changes. Rows render through `virtual_list`, so a long patch builds only the visible rows. Code is coloured by the language of the file path. With `select`, a click on a hunk header selects the hunk and a click or Shift+click selects changed lines, for staging. |
| `Patch`, `Hunk`, `Line`, `LineKind`, `LineId` | `Patch::parse(&str) -> Patch`; `struct Patch { hunks, trailer }`, `line(LineId)`, `changes()`, `hunk_changes(usize)`; `struct Hunk { meta, path, header, old_start, new_start, lines }`; `struct Line { kind, text, old, new }`; `enum LineKind { Context, Removed, Added, Note }` | The parsed unified diff that `diff` renders. |
| `Selection`, `DiffSource` | `struct Selection { hunks: BTreeSet<usize>, lines: BTreeSet<LineId>, .. }`, `contains`, `select_hunk`, `select_line`; `trait DiffSource` for text, `Patch`, and `Signal<Patch>` | What the user selected in a `diff`, and the sources `diff` accepts. |
| `DiffView` | `enum DiffView { Split, Unified }` | Diff view: side-by-side or unified rows. |
| `terminal` | `fn terminal(lines: Signal<Vec<String>>, on_input: impl Fn(String) + 'static) -> NodeBuilder` | A terminal transcript with an input line. |

```rust,no_run
use sigil::widgets::CodeEditBuilder;
use sigil::widgets::ColAlign;
use sigil::widgets::CommandMenuBuilder;
use sigil::widgets::DiffBuilder;
use sigil::widgets::DropdownBuilder;
use sigil::widgets::FuzzyMatch;
use sigil::widgets::Highlight;
use sigil::widgets::HlSpan;
use sigil::widgets::MenuBarBuilder;
use sigil::widgets::MenuBuilder;
use sigil::widgets::ScrollAnchor;
use sigil::widgets::SelectionState;
use sigil::widgets::SubmenuBuilder;
use sigil::widgets::TableBuilder;
use sigil::widgets::TabsBuilder;
use sigil::widgets::VirtualListBuilder;
use sigil::prelude::*;

#[derive(Clone)]
struct File { name: String, size: u64 }

fn tree() -> impl IntoNode {
    let _: DropdownBuilder = dropdown(signal(0), vec!["One".into(), "Two".into()]);
    let _ = combobox(Combobox::new(vec!["red".into(), "blue".into()]));
    let _: MenuBuilder = menu(signal(false));
    let _: Option<(MenuItem, SubmenuBuilder)> = None;
    let _: (MenuBarBuilder, CommandMenuBuilder) = (menu_bar(), command_menu());
    let _: Option<FuzzyMatch> = fuzzy_score("sv", "save file");
    let size = TableColumn::new("size", "Size").numeric().width(ColWidth::Fixed(80.0)).align(ColAlign::End);
    let columns = vec![TableColumn::new("name", "Name").cell(|f: &File| text(f.name.clone()).nowrap()), size];
    let files: TableBuilder<File> = table(TableState::new(vec![File { name: "a".into(), size: 1 }], columns));
    let _ = (Density::Compact, ScrollAnchor::Bottom, SelectionState::new(SelectionMode::Single));
    let list: VirtualListBuilder<String, _> = virtual_list(vec!["a".to_string()], |_, s: &String| list_row(false, s.clone(), ""));
    let (scroll, max_scroll) = (signal(0.0_f32), signal(0.0_f32));
    let list = list.measured().estimate(rows(2.0)).scroll_bind(scroll).max_scroll_bind(max_scroll);
    let list = list.content_height_bind(signal(0.0)).follow_row(signal(0));
    let _ = list_row_with(true, "Inbox", ListRowSlots { marker: Some("*".into()), ..ListRowSlots::default() });
    let _ = (kv_grid([("Name", "Sigil")]), kv_row("Size", text("1 KB")));
    let _: TabsBuilder = tabs().tab("A", || text("a")).tab("B", || text("b"));
    let _: CodeEditBuilder = code_edit(signal(vec!["fn main() {}".into()]), signal((0, 0)));
    let _ = HlSpan { start: 0, end: 2, token: "keyword" };
    let _: Vec<Vec<Highlight>> = highlight("rust", "fn main() {}");
    let _ = code_edit(signal(vec!["let x = 1;".into()]), signal((0, 0))).highlights(signal(hl_spans("rust", "let x = 1;")));
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
| `sidebar` | `fn sidebar(child: impl IntoNode + 'static) -> SidebarBuilder` | A side column. Set its width with `.width(..)`. |
| `status_bar` | `fn status_bar<I: IntoIterator>(children: I) -> NodeBuilder` | A status line. |
| `panel` | `fn panel() -> PanelBuilder` | A titled panel. |
| `card` | `fn card(child: impl IntoNode + 'static) -> CardBuilder`; `.selected(impl Reactive<bool>)`, `.activate(f)` | A bordered card. `selected` gives it the accent border and background; `activate` handles click, Enter, and Space and makes the card a focusable button. Converts into `NodeBuilder`. |
| `stat` | `fn stat(label, value) -> StatBuilder`; `.delta(text, Tone)`, `.caption(text)`, `.trend(values)`, `.range(low, high)`, `.tone(Tone)` | A KPI tile in card chrome: a label, a large value, and an optional toned change line, caption, and sparkline trend. Label and value can be reactive. |
| `tree` | `fn tree<K: TreeKey>(nodes: impl Reactive<Vec<TreeNode<K>>>) -> TreeBuilder<K>` | A tree view for explorers, outlines, and settings. Only rows under expanded nodes are built, and they are virtualized. Click toggles a folder and activates a leaf; arrows move, Left and Right collapse and expand, Home and End jump, Enter activates. |
| `sheet` | `fn sheet(show: Signal<bool>) -> ModalBuilder` | A panel that slides in from a side. |
| `modal` | `fn modal(show: Signal<bool>) -> ModalBuilder` | A modal dialog. While it is open and traps focus, it is marked modal and the rest of the page is hidden from assistive technology. |
| `ModalBuilder` | `struct ModalBuilder`; `close_button(bool)`; `ModalBuilder::new(Signal<bool>)`, `title(text)`, `child(impl IntoNode)`, `footer(impl IntoNode)`, `actions(cancel, confirm)`, `side(Side)`, `escape_closes(bool)`, `dismiss_on_backdrop(bool)`, `trap(bool)`; width with `WidgetLayoutExt::width` | Builder of a modal or sheet. A titled dialog shows a close button that sets `show` to false; `close_button(false)` removes it. Children scroll in a body between the title and the footer, so the title and actions stay visible when the dialog reaches its maximum height. `footer` adds one node to the footer; `actions(cancel, confirm)` adds two. Escape closes the dialog by default. A click on the backdrop closes a `sheet` but not a `modal`, unless `dismiss_on_backdrop` changes it. `trap(false)` lets focus leave the dialog. |
| `SortDir` | `enum SortDir { Ascending, Descending }` | Direction of a table sort. |
| `SortKey` | `struct SortKey { column: &'static str, dir: SortDir }` | Table sort column and direction. |
| `SubmitKey` | `enum SubmitKey { CtrlEnter, Enter }`; `Default` is `CtrlEnter` | The key that runs `TextEditBuilder::submit`. With `Enter`, Shift+Enter inserts a new line. |
| `Side` | `enum Side { Left, Right, Bottom }` | Edge of a sheet. |
| `splitter` | `fn splitter(orientation: Orientation) -> SplitterBuilder` | Resizable panes. |
| `scroll` | `fn scroll(content: NodeBuilder) -> ScrollBuilder` | A scroll area. It measures its content; `.extent(f32 or Signal<f32>)` sets a known content height instead. |
| `section_label` | `fn section_label(title) -> NodeBuilder` | A section heading row. |
| `hint_bar` | `fn hint_bar(keys, right) -> NodeBuilder` | A key hint row. |
| `hint_overlay` | `fn hint_overlay(hints: &[(&str, &str)]) -> HintOverlayBuilder` | An overlay that lists key bindings. |
| `notice_band` | `fn notice_band(body, tone: Tone) -> NodeBuilder` | A full-width notice. |
| `empty_state` | `fn empty_state(title, body) -> NodeBuilder` | A placeholder for empty content. |
| `spacer` | `fn spacer() -> NodeBuilder` | Flexible empty space. |
| `divider_horizontal` | `fn divider_horizontal() -> NodeBuilder` | A horizontal divider. |
| `divider_vertical` | `fn divider_vertical() -> VerticalDivider`; `.height(px)` | A vertical divider that fills the height. `height` gives a fixed tick, such as between toolbar items. |
| `surface` | `fn surface(key: u64) -> NodeBuilder` | A rectangle that the embed host draws. |
| `tooltip` | `fn tooltip(label: impl Into<String>) -> TooltipBuilder` | A hover tooltip. |
| `toast` | `fn toast(message: Signal<Option<String>>) -> NodeBuilder` | One transient message. |
| `toast_queue` | `fn toast_queue(queue: ToastQueue) -> NodeBuilder` | A stack of transient messages. |
| `ToastQueue` | `ToastQueue::new()`; `info`, `success`, `warning`, `error` | Queue of toast messages. |
| `progress` | `fn progress(value: Signal<f32>, max: f32) -> ProgressBuilder` | A progress bar; `.label()` names it, else "Progress". |
| `gauge` | `fn gauge(value: Signal<f32>) -> GaugeBuilder` | A radial gauge. |
| `chart` | `fn chart(values: impl Reactive<Vec<f32>>) -> ChartBuilder`; `ChartBuilder::candles(rows: impl Reactive<Vec<(f32, f32, f32, f32)>>)`, `.x_range(Signal<Range<f32>>)`, `.crosshair()` | A bar, line, or candle chart. A line with more points than the plot has pixel columns keeps the low and high point of each column, so 100k points draw as one short path. `candles` takes `(open, high, low, close)` rows. `x_range` binds the visible range in point units: the wheel zooms around the pointer and a drag pans. `crosshair` draws rules and the value under the pointer. A signal or closure source draws the chart again when it changes, like `sparkline`. The chart fills its parent's width unless `.width()` is set; at small heights it thins x-axis labels and hides the axis row before the plot gets too small. |
| `sparkline` | `fn sparkline(values: impl Reactive<Vec<f32>>) -> SparklineBuilder` | A sparkline. By default, one block glyph per value on one text row. A signal or closure source draws the graph again when it changes. |
| `heatmap` | `fn heatmap<F: Fn(usize, usize) -> Rgba8>(grid_w, grid_h, color: F) -> NodeBuilder` | A grid of coloured cells, drawn as one image that fills its box. |
| `dot` | `fn dot(tone: Tone) -> DotBuilder` | A status dot. |
| `avatar` | `fn avatar(initials) -> NodeBuilder` | Initials in a circle. |
| `spinner` | `fn spinner() -> NodeBuilder` | A busy indicator named "Loading". |
| `icon` | `fn icon(glyph: impl IntoIconSource) -> IconBuilder` | An icon: a bundled `Icon`, such as `icon(Icon::Folder)`, or a text glyph. |
| `Icon` | `enum Icon` (105 variants); `Icon::ALL`, `name()`, `svg()` | The bundled Lucide icon set (ISC). `name` is the kebab-case file name and `svg` the SVG source (feature `scene`). |
| `path` | `const fn path(width: u32, height: u32) -> PathNode` | A vector path. |
| `PathNode` | `move_to`, `line_to`, `quad_to`, `cubic_to`, `close`, `fill`, `fill_linear(start, end, stops: Vec<(f32, Rgba8)>)`, `fill_radial(center, radius, stops)`, `stroke`, `non_scaling_stroke` | Builder of a vector path. `fill_linear` and `fill_radial` fill with a gradient. The host rasterizes the path at the laid-out size times the device scale. `non_scaling_stroke` keeps the stroke width in logical px under any fit, with round joins and caps. |
| `image_rgba` | `fn image_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<ImageBuilder>` | An image from RGBA bytes. |
| `image_bytes` | `fn image_bytes(encoded: &[u8]) -> Option<ImageBuilder>` | An image from PNG or JPEG bytes (feature `image-decode`). |

```rust,no_run
use sigil::kernel::ImageBuilder;
use sigil::widgets::ChartBuilder;
use sigil::widgets::ChartMark;
use sigil::widgets::DotBuilder;
use sigil::widgets::GaugeBuilder;
use sigil::widgets::HintOverlayBuilder;
use sigil::widgets::IconBuilder;
use sigil::widgets::icon_svg;
use sigil::widgets::spinner_frame;
use sigil::widgets::ModalBuilder;
use sigil::widgets::PanelBuilder;
use sigil::widgets::PathCmd;
use sigil::widgets::PathColor;
use sigil::widgets::PathFill;
use sigil::widgets::ProgressBuilder;
use sigil::widgets::ScrollBuilder;
use sigil::widgets::SparkMark;
use sigil::widgets::SparklineBuilder;
use sigil::widgets::SplitterBuilder;
use sigil::widgets::ToastEntry;
use sigil::widgets::ToastLevel;
use sigil::widgets::TooltipBuilder;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let open = signal(false);
    let queue = ToastQueue::new();
    queue.info("Saved");
    let _: Option<ToastEntry> = None;
    let dialog: ModalBuilder = modal(open);
    let _: ModalBuilder = sheet(open);
    let _ = Side::Right;
    let _: SplitterBuilder = splitter(Orientation::Horizontal);
    let _: ScrollBuilder = scroll(container()).extent(400.0_f32);
    let _: PanelBuilder = panel();
    let _: HintOverlayBuilder = hint_overlay(&[("q", "quit")]);
    let _ = (section_label("Files"), hint_bar("q quit", "v0.1"), notice_band("Offline", Tone::Warn));
    let _ = (empty_state("No files", "Add one"), divider_horizontal(), divider_vertical());
    let _ = (surface(1), toast(signal(None)), toast_queue(queue), ToastLevel::Error);
    let _: TooltipBuilder = tooltip("Help");
    let _: (ProgressBuilder, GaugeBuilder) = (progress(signal(0.3), 1.0).tone(Tone::Warn), gauge(signal(0.5)));
    let _: ChartBuilder = chart([1.0, 2.0, 3.0]).line();
    let _: SparklineBuilder = sparkline([1.0, 4.0, 2.0]).range(0.0, 5.0).tone(Tone::Warn);
    let _: SparklineBuilder = sparkline([1.0, 4.0]).mark(SparkMark::Braille).rows(3).tones([None, Some(Tone::Error)]);
    let _ = (ChartMark::Bar, PathCmd::Close, PathFill::Solid(Rgba8 { r: 0, g: 0, b: 0, a: 255 }));
    let _ = heatmap(4, 4, |x, y| Rgba8 { r: (x * 60) as u8, g: (y * 60) as u8, b: 0, a: 255 });
    let _: DotBuilder = dot(Tone::Success);
    let _ = (avatar("TG"), spinner(), spinner_frame(3));
    let _: IconBuilder = icon("*");
    let _: IconBuilder = icon_svg(sigil::icons::builtin::GEAR_SVG).tone(Tone::Primary);
    let _: PathNode = path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0).stroke(PathColor::Token(sigil::style::Token::Gray11), 1.5);
    let _: PathNode = path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0).close().fill(Rgba8 { r: 0, g: 0, b: 0, a: 255 });
    let _: Option<ImageBuilder> = image_rgba(1, 1, vec![0, 0, 0, 255]);
    let _ = spacer();
    let work = sidebar(card(text("Card"))).width(200.0);
    app_shell(text("Top"), work, status_bar([text("Ready")]), Some(dialog))
}

#[cfg(feature = "image-decode")]
fn decoded(bytes: &[u8]) -> Option<ImageBuilder> { image_bytes(bytes) }

fn main() {
    let _ = tree;
}
```

### Drag and drop

| Name | Signature | Purpose |
| --- | --- | --- |
| `DragDropExt` | `fn drag_source(self, state, payload)`, `fn drop_target(self, state, accepts, on_drop)` | Make a node a drag source or a drop target. |
| `DragState` | `DragState::new()` | Shared state of one drag interaction. |
| `drag_ghost` | `fn drag_ghost<T, N>(state: DragState<T>, render: impl Fn(&T) -> N + 'static) -> NodeBuilder` | The node that follows the pointer. |
| `axis_drag` | `fn axis_drag<S>(node, axis: Axis, snapshot, on_delta, on_end) -> NodeBuilder` | Drag along one axis. |

```rust,no_run
use sigil::widgets::DRAG_THRESHOLD_PX;
use sigil::widgets::DROP_TARGET_CLASS;
use sigil::prelude::*;
use sigil::widgets::Axis;

fn tree() -> impl IntoNode {
    let drag: DragState<u32> = DragState::new();
    let source = container().drag_source(drag.clone(), 7);
    let target = container().drop_target(drag.clone(), |n: &u32| *n > 0, |n| println!("{n}"));
    let ghost = drag_ghost(drag, |n: &u32| text(n.to_string()));
    let width = signal(200.0_f32);
    let handle = axis_drag(container(), Axis::X, move || width.peek(), move |start, dx| width.set(start + dx), || {});
    let _ = (DRAG_THRESHOLD_PX, DROP_TARGET_CLASS);
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
| `tasks::panic_message` | `fn panic_message(payload: &(dyn Any + Send)) -> &str` | Text of a panic payload; `"<non-string panic payload>"` when it is not a string. |
| `animation_runtime::install_animators_themed` | `fn install_animators_themed(atoms: &[StyleAtom], state_style: StateStyle, component_id: ComponentId, theme: &Theme) -> AttachedAnimators` | Like `install_animators`, with token colours resolved against `theme`. |
| `animation_runtime::AttachedColorAnimator` | `struct AttachedColorAnimator { kind: ColorAtomKind, channels: [Signal<f32>; 4], animators: [Animator; 4] }` | One animated colour atom: one signal and animator per RGBA channel. |
| `animation_runtime::lerp_rgba8` | `fn lerp_rgba8(a: Rgba8, b: Rgba8, t: f32) -> Rgba8` | Mix two sRGB colours in linear light. `t` is clamped to 0..=1. |
| `animation_runtime::linear_channel_to_srgb` | `fn linear_channel_to_srgb(v: f32) -> u8` | Encode a linear channel in 0..=1 as an sRGB byte. |
| `a11y::RecordingBridge` | `struct RecordingBridge { pending_actions: VecDeque<(NodeId, ActionRequest)> }`; `new()`, `action(target, request)`, `last_published()`, `published_count()` | An `A11yBridge` that records published trees and queues actions, for tests. |
| `a11y::BoundsLookup` | `type BoundsLookup<'a> = &'a dyn Fn(NodeId) -> Option<Rect>` | Node bounds source for `a11y::tree`. |
| `reactive::next_interval_deadline` | `fn next_interval_deadline() -> Option<Instant>` | Earliest next fire time of the registered intervals. An embed host loop sleeps until it. |
| `AppConfig::inline` | `const fn inline(self, yes: bool) -> Self` | Run the terminal host in the normal screen below the prompt instead of the alternate screen. |
| `AppConfig::reduced_motion` | `const fn reduced_motion(self, reduced: bool) -> Self` | Override the OS reduced-motion setting for `MotionPrefs`. |
| `AppConfig::click` | `const fn click(self, click: hal::ClickConfig) -> Self` | Override the double-click interval and distance. |
| `AppConfig::task_workers` | `const fn task_workers(self, workers: tasks::TaskWorkers) -> Self` | Set the worker threads of the task runtime. Default `Auto` on every host. |
| `events::ClickConfig` | `struct ClickConfig { interval: Duration, distance: f32 }`; `current()`, `set(config)`, `is_repeat(&self, elapsed, from, to) -> bool` | Double-click limits. Default 450 ms and 8 px; `sigil::run` takes the OS interval on macOS and GNOME. |
| `AppConfig::terminal_hooks` | `fn terminal_hooks(self, hooks: terminal::Hooks<'static>) -> Self` | Terminal host hooks for `sigil::run`: `external_events`, `after_frame`, `region_rows`. The window host ignores them. |
| `terminal::Hooks::region_rows` | `region_rows: Option<Box<dyn FnMut() -> u16>>` | Height of the inline live region each frame, clamped to 1..=terminal rows. Used only with `inline(true)`. |
| `terminal::print_above` | `fn print_above(text: &str) -> io::Result<()>` | Write lines above the inline live region, into scrollback. Returns `PrintAboveError` in an `io::Error` when no inline host is live on this thread. |
| `AppConfig::terminal_access` | `const fn terminal_access(self, access: terminal::Access) -> Self` | Screen-reader options for the terminal host. The window host ignores them. |
| `terminal::Access` | `struct Access { focus_cursor: FocusCursor, status: Duration }`; `Default` is `Follow` and 5 s | Where the terminal cursor goes when no caret shows, and how long an announcement stays on the full-screen status row. `Screen::set_access(access)` applies it to a `Screen`. |
| `terminal::FocusCursor` | `enum FocusCursor { Follow, Off }` | `Follow` moves the hidden terminal cursor to the focused node's label when no text caret is visible, so screen readers read it. `Off` leaves it where it is. The cursor shows only at a text caret. |
| `AppConfig::mouse_tracking` | `const fn mouse_tracking(self, tracking: terminal::MouseTracking) -> Self` | Mouse reports the terminal host asks for at start. Without it, the host keeps the last `set_mouse_tracking` value, `Drags` by default. The window host ignores it. |
| `terminal::MouseTracking` | `enum MouseTracking { Off, Clicks, Drags, Motion }`; `Default` is `Drags` | Terminal mouse reports, all in SGR form (1006). `Clicks` sends presses, releases, and the wheel (1000). `Drags` adds motion while a button is held (1002). `Motion` adds motion with no button held (1003), for hover. Every change and the exit first reset all four modes. |
| `AppConfig::resizable` | `const fn resizable(self, yes: bool) -> Self` | Let the user resize the window (feature `desktop`). `false` fixes the window at its configured size. |
| `AppConfig::esc_exits` | `const fn esc_exits(self, yes: bool) -> Self` | Exit the app on Escape. `exits_on_esc(&self) -> bool` reads the setting. |
| `AppConfig::on_panic` | `const fn on_panic(&self) -> PanicPolicy` | The configured panic policy. |
| `AppConfig::suspend_on` | `fn suspend_on(self, shortcut: impl Into<Option<Shortcut>>) -> Self` | The key that suspends the terminal host (job control); the default is Ctrl+Z. The host restores the terminal and stops; `fg` resumes it and redraws the screen. `None` turns it off and delivers the key to the app. Unix only; the window host ignores it. |
| `kernel::Cx::announce_assertive` | `fn announce_assertive(&mut self, text: impl Into<String>)` | Like `announce`, with assertive politeness: screen readers interrupt current speech. |
| `kernel::Cx::flush_announcements` | `fn flush_announcements(&mut self)` | Move the text that `kernel::announce` queued into the live region. `Host::frame` calls it every frame. |
| `kernel::announce` | `fn announce(text: impl Into<String>)` | Queue a polite screen-reader announcement; the next frame posts it to the live region. |
| `kernel::announce_assertive` | `fn announce_assertive(text: impl Into<String>)` | Queue an assertive announcement that interrupts the screen reader. |
| `kernel::Cx::dump_tree` | `fn dump_tree(&self, root: NodeId) -> String` | Indented text of the node tree below `root`, for debugging. |
| `kernel::Cx::describe` | `fn describe(&self, id: NodeId) -> String` | A short label for one node: its role, then its accessible name, its text, or the nearest named ancestor. Profiler attributions use it. |
| `reactive::debug::dump_owner_tree` | `fn dump_owner_tree() -> String` | Indented text of every reactive owner with its name and source location, for debugging leaks. |
| `reactive::debug::dump_subtree` | `fn dump_subtree(owner: OwnerId) -> String` | `dump_owner_tree` for one owner and its children. |
| `animation::Spring::with_epsilons` | `fn with_epsilons(self, value: f32, velocity: f32) -> Self` | Rest thresholds of a spring. Both must be positive; it panics otherwise. |
| `animation::Tween::restart` | `const fn restart(&mut self)` | Set the elapsed time back to zero. |
| `bidi::paragraph_levels` | `fn paragraph_levels(input: &str) -> SmallVec<[u8; 64]>` | Unicode bidi embedding level of each byte of `input`. |
| `app::menu::set_menu_bar` | `fn set_menu_bar(menu: app::menu::Menu)` | Install the native menu bar. The window host installs it on the next frame (feature `menu`). |
| `app::menu::accelerator` | `fn accelerator(shortcut: &Shortcut) -> Option<app::menu::KeyAccelerator>` | Native menu accelerator for a `Shortcut`, so one key definition drives both the key binding and the menu hint. `NativeMenuItem::with_id(id, label, enabled, accelerator)` takes muda's code-based `Accelerator`, not this type: build the item with `None`, then call `item.set_key_accelerator(accelerator(&shortcut))`. Returns `None` for named keys other than Enter, Tab, Escape, Backspace, Delete, Insert, Home, End, PageUp, PageDown, the arrow keys, and F1–F12 (feature `menu`). |
| `app::tray::set_tray` | `fn set_tray(builder: tray_icon::TrayIconBuilder)` | Install a system tray icon. The window host builds it on the next frame (feature `tray`). |
| `scene::SceneBuilder::with_layer` | `fn with_layer(&mut self, opacity: f32, body: impl FnOnce(&mut Self))` | Draw `body` into a layer composited at `opacity`. |
| `scene::SceneBuilder::with_transform` | `fn with_transform(&mut self, transform: Transform, body: impl FnOnce(&mut Self))` | Draw `body` under `transform`. |
| `app::dialog::FileDialog` | `FileDialog::new()`; `filter(name, extensions)`, `directory(dir)`, `file_name(name)`; `open_file()`, `save_file()`, `pick_folder() -> Picked<PathBuf>`, `open_files() -> Picked<Vec<PathBuf>>` | File picker (feature `dialog`). The result signal starts `Pending` and resolves without blocking the frame loop. The in-app picker runs on every host; `native-dialog` opens the OS dialog on the window host. |
| `style::palette::Palette` | `struct Palette { background, text, primary, success, warning, danger: Rgba8 }`; `obsidian()`, `terminal()`, `aetherlight()`, `voltglyph()`, `mythril()`, `frostkeep()`, `vellum()`, `embergrove()`, `verdigris()`, `faewine()`, `crystalline()`, `spellforge(accent: Rgba8)` | Named six-colour palettes. `spellforge` derives a palette from one accent colour. |
| `style::scale` | `const fn {slate, blue, green, amber, ruby, cyan, iris}_{light, dark}() -> ColorScale` | Twelve-step colour scales for `Theme`. |
| `style::Theme::with_gray_scale` | `with_gray_scale`, `with_accent_scale`, `with_success_scale`, `with_warn_scale`, `with_error_scale`, `with_info_scale` (each `const fn(self, scale: ColorScale) -> Self`) | Replace one colour scale of a theme. |
| `style::theme::font_family_hint` | `const fn font_family_hint(variant: Variant) -> FontFamilyHint` | The font family a theme variant asks for. |
| `layout::LayoutStyle` | `grid_columns(Vec<TrackSize>)`, `grid_rows(Vec<TrackSize>)`, `grid_area(col, row)`, `grid_dense()`, `max_height(impl Into<Dimension>)`, `with_layout(f: impl FnOnce(&mut StyleData))` | Grid tracks and placement, and direct access to the layout data. |
| `style::Metrics::icon_size` | `fn icon_size(self) -> f32` | Icon size in points, or one cell on the terminal host. |
| `style::LengthPercentage::px_or_zero` | `const fn px_or_zero(self) -> f32` | The pixel value, or 0 for a non-pixel size. `Dimension` has the same method. |
| `style::binding::StyleBinding::set_animator_overlay` | `fn set_animator_overlay(&mut self, overlay: Option<Box<dyn AnimatorOverlay>>)` | Replace the animated values that override the resolved style. |
| `text::add_fallback_font` | `fn add_fallback_font(bytes: Vec<u8>, face_index: u32) -> Result<(), TextError>` | Add a fallback face to every text engine. Shaping uses it for characters the primary faces lack. Layout measures text again on the next frame. |
| `text::load_font_url` | `fn load_font_url(url: &str, face_index: u32) -> Signal<Status<(), FontLoadError>>` (wasm32 only) | Fetch a font file and add it with `add_fallback_font`. The signal turns `Ok(())` when the face is in use. |
| `text::FontLoadError` | `enum FontLoadError { Fetch(String), Http(u16), InvalidFont }` (wasm32 only) | Why `load_font_url` failed. |
| `text::OwnedTextEngine` | `register_font(bytes: Vec<u8>, face_index: u32) -> Result<(), TextError>`, `clear_cache()` | The text engine of a host. App code reaches the shared engine only through `OwnedTextEngine::thread()`, an `Rc`, so it cannot call `register_font`; use `app::set_default_fonts` to replace a face or `text::add_fallback_font` to add one. |
| `layout::Dimension` | `enum Dimension` | A length: px, percent, text length, or auto. |
| `layout::LengthPercentage` | `enum LengthPercentage` | A px or percent length. |
| `style::Dimension` | `enum Dimension { Px(Px), Full, Fraction(u16), Auto }` | A size in a style atom. |
| `patterns` | `mod patterns` | The module of the frame and row patterns. |
| `clipboard::paste` | `fn paste() -> AppResult<String>` | Read text from the installed clipboard. |

## Module `a11y`

| Name | Signature | Purpose |
| --- | --- | --- |
| `a11y::AccessKitBridge`, `a11y::bridge::AccessKitBridge` | `struct AccessKitBridge<F: FnMut(TreeUpdate)>`; `const fn new(action_rx: crossbeam_channel::Receiver<ActionRequest>, publish_sink: F) -> Self` | Implements `A11yBridge` over AccessKit. It passes each tree update to the sink and reads action requests from the channel. |
| `a11y::BridgeError`, `a11y::bridge::BridgeError` | `struct BridgeError;` | Error type of `AccessKitBridge`. It reports that the bridge rejected an update. |

## Module `animation`

| Name | Signature | Purpose |
| --- | --- | --- |
| `animation::SpringPreset` | `enum SpringPreset { Gentle, Smooth, Snappy, Bouncy, Stiff }` | Spring constants. |
| `animation::Easing` | `enum Easing` | Tween easing curve. |
| `animation::CubicBezier` | `CubicBezier::new(x1, y1, x2, y2)` | Custom easing curve. |
| `animation::Keyframe` | `Keyframe::new(offset: f32, value: f32)` | One keyframe. |
| `animation::Keyframes` | `Keyframes::new(frames, duration: Duration)`; `repeat_forever()` | A keyframe animation. `repeat_forever()` loops it with no iteration limit. |
| `animation::AnimationDirection` | `enum AnimationDirection` | Play direction of keyframes. |
| `animation::AnimationFill` | `enum AnimationFill { None, Forwards, Backwards, Both }` | Value before and after keyframes. |

## Module `animation_runtime`

| Name | Signature | Purpose |
| --- | --- | --- |
| `animation_runtime::AnimatorStatus` | `enum AnimatorStatus { Running, Settled }` | Tells whether an `Animator` still moves or has reached its target. |
| `animation_runtime::AttachedAnimator`, `animation_runtime::binding_animate::AttachedAnimator` | `struct AttachedAnimator { kind: NumericAtomKind, signal: Signal<f32>, animator: Animator, .. }` | One numeric style atom that a component animates: the atom kind, the signal that holds the current value, and its animator. |
| `animation_runtime::srgb_channel_to_linear` | `fn srgb_channel_to_linear(c: u8) -> f32` | Converts one sRGB 8-bit channel to a linear value in 0.0 to 1.0. |
| `animation_runtime::presets` | `mod presets { const SMOOTH, GENTLE, SNAPPY, BOUNCY, STIFF, FAST, DEFAULT, SLOW: TransitionId }` | Named `TransitionId` values that `animation_runtime::config` maps to spring and tween configs. |
| `animation_runtime::active_count` | `fn active_count() -> usize` | Returns the number of animators on this thread that are not paused and not settled. |
| `animation_runtime::Animator` | `Animator::new(signal: Signal<f32>, config: AnimationConfig)`; `pause()`, `resume()` | Animate a signal toward targets. |
| `animation_runtime::AnimationConfig` | `enum AnimationConfig { Spring { preset }, Tween { ms, easing } }` | Spring or tween settings. |
| `animation_runtime::Playback` | `Playback::new(value: Signal<f32>, frames: Keyframes)`; `pause()`, `resume()`, `restart()` | Play keyframes into a signal. `restart()` seeks to the start. |
| `animation_runtime::MotionPrefs` | `MotionPrefs::reduced() -> bool`, `MotionPrefs::set(bool)` | Reduced-motion preference. `sigil::run` sets it from `AppConfig::reduced_motion`, else from the OS (macOS accessibility setting, GNOME `enable-animations`). |

## Module `app`

| Name | Signature | Purpose |
| --- | --- | --- |
| `app::menu::AboutMetadata`, `app::menu::CheckMenuItem`, `app::menu::ContextMenu`, `app::menu::IconMenuItem`, `app::menu::KeyAccelerator`, `app::menu::Menu`, `app::menu::MenuEvent`, `app::menu::MenuId`, `app::menu::NativeMenuItem`, `app::menu::PredefinedMenuItem`, `app::menu::Submenu` | re-exports of `muda` types | Native menu types; an app needs no direct `muda` dependency. `NativeMenuItem` is muda's `MenuItem`, renamed to stay distinct from `widgets::MenuItem`. Build the menu passed to `app::menu::set_menu_bar`; clicks arrive as `MenuId` on `app::menu::events`. |
| `app::tray::Icon`, `app::tray::TrayIcon`, `app::tray::TrayIconEvent` | re-exports of `tray_icon` types | Tray types from the `tray-icon` crate. `Icon` is the tray image, `TrayIcon` is the installed tray, and `TrayIconEvent` arrives on `app::tray::events`. |
| `app::CellWidth` | `enum CellWidth { Unicode, Scalar { joiner: usize, nonjoiner: usize, keycap: usize } }` | Selects how the terminal host counts cell width: Unicode East Asian width, or fixed widths for joiners, non-joiners, and keycaps. Default is `Unicode`. |
| `app::set_cell_width`, `app::cell_width` | `fn set_cell_width(width: CellWidth)`; `fn cell_width(text: &str) -> usize` | Set the cell width rule for this thread, and return the number of terminal cells that a string occupies under that rule. |
| `app::Ambiguous`, `app::set_ambiguous` | `enum Ambiguous { Narrow, Wide }`; `fn set_ambiguous(width: Ambiguous)` | Width of East Asian ambiguous characters for this thread. `Wide` counts them as two cells, as CJK terminals do. Default is `Narrow`. |
| `app::cell_text_lines` | `fn cell_text_lines(text: &str, max_cells: usize) -> Vec<(usize, usize)>` | Wraps text to `max_cells` terminal cells and returns the byte range of each line. `0` means no wrap. |
| `app::FrameInfo` | `#[non_exhaustive] pub struct FrameInfo { dirty: bool, animating: bool, layout_ok: bool }` | State of the current frame that `Host::frame` passes to the paint hook. |
| `app::FrameOutput` | `struct FrameOutput { exit: bool, intervals: usize, animating: bool, layout: Result<(), LayoutError> }` | Result of `Host::frame`: exit request, number of intervals fired, whether animations still run, and the layout result. |
| `app::FrameHooks`, `app::FrameInputHook`, `app::FrameLayoutHook`, `app::FramePaintHook` | `struct FrameHooks<'a> { input: Option<FrameInputHook<'a>>, after_layout: Option<FrameLayoutHook<'a>>, paint: Option<FramePaintHook<'a>> }`; `type FrameInputHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter) -> Option<(f32, f32)> + 'a>`; `type FrameLayoutHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter) + 'a>`; `pub type FramePaintHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter, &FrameInfo) + 'a>` | Callbacks that an embedding host passes to `Host::frame` to feed input, act after layout, and paint. The input hook returns the viewport when input changed it (a resize), and the frame lays out at that size; `None` keeps the viewport passed to `frame`. |
| `app::EngineGeometry` | `struct EngineGeometry`; `fn new() -> Self` | `TextGeometry` implementation backed by the thread text engine. It gives hit tests and caret positions for text nodes. |
| `app::DefaultTextMeasure` | `struct DefaultTextMeasure`; `fn new() -> Self` | `TextMeasure` implementation backed by the thread text engine. Layout uses it to size text nodes. |
| `app::is_grid` | `fn is_grid() -> bool` | `true` on a cell grid (terminal) host. |
| `app::viewport_size` | `fn viewport_size() -> Signal<LogicalSize>` | The viewport size in logical px. A terminal cell is `Host::CELL` px. |
| `app::display_scale` | `fn display_scale() -> Signal<f32>` | Physical pixels per logical px of the window, user zoom included. It is 1.0 outside a host. Size raster work such as image decodes with it. |
| `app::set_theme` | `fn set_theme(theme: Theme)` | Replace the active theme at the next frame. |
| `app::parse_theme` | `fn parse_theme(input: &str) -> Option<Theme>` | Parse a theme TOML document. |
| `app::watch_theme` | `fn watch_theme(path: impl Into<PathBuf>) -> AppResult<ThemeWatcher>` | Reload a theme file when it changes. |
| `app::ThemeWatcher` | `struct ThemeWatcher` | Guard of `watch_theme`. Drop it to stop the watch. |
| `app::open` | `fn open(target: impl AsRef<OsStr>) -> AppResult<()>` | Open a file or URL with the OS handler (feature `desktop`). |
| `app::dialog` | `mod dialog` | File and message dialogs. No call blocks the frame loop. Feature `dialog` gives the API and an in-app modal that works on every host and needs no desktop dependencies. Feature `native-dialog` adds the OS dialog on the window host, run on the task runtime; the terminal host, or a Linux session without `DISPLAY` or `WAYLAND_DISPLAY`, still uses the in-app modal. The in-app modal shows the directory path, a list with `..` and folders first, the extension filter applied, a name field in save mode, and Cancel and Open buttons. Arrows move, Enter opens a folder or picks a file, Backspace goes up, and Escape cancels. The caller code is the same on both hosts. |
| `app::tray` | `mod tray` | System tray icon (feature `tray`). |
| `app::menu` | `mod menu` | Native menu bar (feature `menu`). |
| `app::Player` | `struct Player` | Audio file player (feature `audio`). |
| `app::Session` | `struct Session` | Child process in a pseudo-terminal (feature `pty`). |
| `app::ErrorKind` | `enum ErrorKind` | Class of an `Error`. |
| `app::PanicPolicy` | `enum PanicPolicy { Isolate, Abort }` | What a panic in a handler does. |
| `app::HostCaps` | `struct HostCaps { hyperlinks: bool }`; `HostCaps::current()`, `HostCaps::window()` | What the running host can show. The terminal host fills it from the detected terminal. |
| `app::AccessibilityTree` | `type AccessibilityTree = accesskit::TreeUpdate` | Accessibility tree of an embedded tree. |
| `app::WindowCommand` | `enum WindowCommand` | Close, minimize, maximize, drag, or zoom the window. Feature `desktop`. |
| `app::ResizeDirection` | `enum ResizeDirection` | Edge or corner for `WindowCommand::DragResize`. Feature `desktop`. |
| `app::WindowOpts` | `struct WindowOpts` | Window options that `AppConfig` collects. Feature `desktop`. |
| `app::Anchor` | `enum Anchor { BottomCenter { margin: f32 } }` | Screen anchor of an overlay window. Feature `desktop`. |
| `app::DefaultFonts` | `struct DefaultFonts` | Font bytes for the sans, mono, icon, and display faces. |
| `app::set_default_fonts` | `fn set_default_fonts(fonts: DefaultFonts)` | Replace the bundled fonts. |
| `app::use_system_fonts` | `fn use_system_fonts()` | Use the operating system fonts. |
| `app::TextTuning` | `struct TextTuning` | Text rasterisation settings. |
| `app::text_tuning` | `fn text_tuning() -> TextTuning` | Read the text rasterisation settings. |
| `app::set_text_tuning` | `fn set_text_tuning(tuning: TextTuning)` | Set the text rasterisation settings. |

## Module `bidi`

| Name | Signature | Purpose |
| --- | --- | --- |
| `bidi::visual_order` | `fn visual_order(levels: &[u8]) -> Vec<usize>` | Returns the visual order of runs from their bidi embedding levels. |
| `bidi::Step` | `enum Step { To(usize), Edge, Logical }` | Result of a visual caret move: a new byte offset, the line edge, or a fall back to logical movement. |

## Module `clipboard`

| Name | Signature | Purpose |
| --- | --- | --- |
| `clipboard::NoopClipboard` | `struct NoopClipboard;` | `Clipboard` implementation used when no clipboard is installed. `set` returns an error. |
| `clipboard::copy` | `fn copy(text: &str) -> AppResult<Delivery>` | Write text to the installed clipboard. |
| `clipboard::image`, `clipboard::copy_image` | `fn image() -> AppResult<Option<ImageSource>>`; `fn copy_image(image: &ImageSource) -> Result<Delivery, ClipError>`; `Clipboard::set_image` (feature `clipboard-image`) | Read an image from the clipboard, `None` when it holds no image, or write one. The web clipboard has no image write and returns a `ClipError`. |
| `clipboard::ClipError` | `ClipError::new(msg)` | A clipboard back end failed. |
| `clipboard::OsClipboard` | `struct OsClipboard` | The OS clipboard (feature `clipboard`). |
| `clipboard::install` | `fn install(clip: impl Clipboard + 'static)` | Replace the installed clipboard. |
| `clipboard::Clipboard` | `trait Clipboard { fn set(&self, &str) -> Result<Delivery, ClipError>; fn get(&self) -> Option<String>; }` | A clipboard back end. |
| `clipboard::Delivery` | `enum Delivery { Confirmed, Requested }` | Whether the OS took the text. |

```rust,no_run
use sigil::clipboard::ClipError;
use sigil::clipboard::OsClipboard;
use sigil::clipboard::copy;
use sigil::prelude::*;
use sigil::clipboard::Clipboard;
use sigil::clipboard::Delivery;
use sigil::clipboard::install;

struct Memory;

impl Clipboard for Memory {
    fn set(&self, _text: &str) -> Result<Delivery, ClipError> { Err(ClipError::new("read-only")) }
    fn get(&self) -> Option<String> { Some("text".into()) }
}

fn main() -> AppResult<()> {
    install(Memory);
    let _: Delivery = copy("hello")?;
    let _: String = sigil::clipboard::paste()?;
    let _: Option<OsClipboard> = None;
    Ok(())
}
```

## Module `events`

| Name | Signature | Purpose |
| --- | --- | --- |
| `events::focus_from_keyboard` | `fn focus_from_keyboard() -> bool` | Whether the last focus change came from the keyboard, which decides whether a focus ring shows. |
| `events::EventHandlers` | `struct EventHandlers` | The handler set of a node. |
| `events::request_reveal` | `fn request_reveal(id: NodeId)` | Scroll each enclosing `scroll` so a node is in view after the next layout. |
| `events::FocusIntent` | `enum FocusIntent { Up, Down, Left, Right }` | Directional focus move. |
| `events::SelectableText` | `struct SelectableText` | Node extension that marks a text region as selectable. `NodeEventsExt::selectable_text(true)` attaches it. |
| `events::request_window_command` | `fn request_window_command(command: WindowCommand)` | Send a command to the window. Feature `desktop`. |
| `events::PhysicalKey` | `keyboard_types::Code` | A key position on the keyboard. |
| `events::ImeEvent` | `enum ImeEvent { Enabled, Preedit, Commit, Disabled }` | Input method event. |
| `events::ScrollEvent` | `struct ScrollEvent` | A wheel or trackpad scroll. Each `on_scroll` handler sees `position` relative to its own node, as pointer handlers do. |
| `events::ScrollDelta` | `enum ScrollDelta { Pixels(Point), Lines(Point) }` | Scroll amount. |
| `events::ScrollPhase` | `enum ScrollPhase { Started, Moved, Ended, Cancelled }` | Scroll gesture phase. |
| `events::FileDropEvent` | `struct FileDropEvent` | A file drag over or onto the window. The host reports no drop point, so the event goes to the deepest `file_drop` handler under the last known pointer position, else on the focused node's path; with neither, every handler sees it. |
| `events::FileDropPhase` | `enum FileDropPhase { Hovered, Dropped, Cancelled }` | File drop phase. |
| `events::WindowEvent` | `enum WindowEvent` | Resize, focus, and close events of the window. |
| `events::KeyEventInit`, `hal::events::KeyEventInit` | `struct KeyEventInit { key: LogicalKey, physical: PhysicalKey, modifiers: ModifiersState, repeat: bool, phase: KeyPhase, text: Option<SmolStr> }` | Fields for `KeyEvent::new`. |
| `events::ScrollEventInit`, `hal::events::ScrollEventInit` | `struct ScrollEventInit { position: Point, delta: ScrollDelta, modifiers: ModifiersState, phase: ScrollPhase }` | Fields for `ScrollEvent::new`. |
| `events::EventRouter` | `EventRouter::new()` | Routes shell events to an embedded tree. |
| `events::ShellEvent` | `enum ShellEvent { Pointer, Key, Ime, Scroll, Gesture(GestureEvent), FileDrop, Paste(String), Window }` | An input event from the host. The terminal host sends a bracketed paste as one `Paste`. |
| `events::PasteEvent` | `struct PasteEvent { pub text: String, .. }`; `PasteEvent::new(text)`, `stop_propagation()`, `propagation_stopped() -> bool` | Pasted text routed to the focused node, then to each ancestor until a handler stops it. `NodeEventsExt::on_paste(f)` receives it. Text fields insert it as one edit, the same as a clipboard paste. |
| `events::PointerEventInit` | `struct PointerEventInit` | Fields to build a `PointerEvent`. `PointerEvent::with_contact(pointer_id, pressure, tilt)` sets the contact data. |
| `events::PointerTilt` | `struct PointerTilt { x: f32, y: f32 }` | Pen tilt in degrees. |
| `events::GestureEvent`, `events::GestureEventInit`, `events::GestureKind` | `struct GestureEvent { kind, phase: ScrollPhase, position, modifiers, .. }`; `enum GestureKind { Pinch(f64), Rotate(f32), Pan(Point), DoubleTap }`; `stop_propagation()` | A trackpad or touch gesture. It goes to the node under its position and bubbles like a scroll; `NodeEventsExt::on_gesture(f)` receives it. |
| `events::ShortcutParseError` | `enum ShortcutParseError { Empty, UnknownModifier(String), UnknownKey(String) }` | Why a shortcut string did not parse. |

## Module `geometry`

| Name | Signature | Purpose |
| --- | --- | --- |
| `geometry::Dim` | `enum Dim { Fixed(f32), MinContent, MaxContent }` | One axis of a size constraint. |
| `geometry::LogicalSize` | `struct LogicalSize { width: f32, height: f32 }` | A size in logical px or cells. |
| `geometry::PhysicalSize` | `struct PhysicalSize { width: u32, height: u32 }`; `const fn new(width: u32, height: u32) -> Self` | Size in device pixels. |
| `geometry::Scale`, `hal::size::Scale` | `struct Scale(f32)`; `const ONE`; `fn new(scale: f32) -> Self`; `const fn factor(self) -> f32`; `fn to_physical(self, logical: LogicalSize) -> PhysicalSize` | Device scale factor. `new` replaces a non-finite or non-positive value with `ONE`. |


## Module `hal`

| Name | Signature | Purpose |
| --- | --- | --- |
| `hal::Renderer`, `renderer::Renderer` | `trait Renderer { type Error; type Surface; type WindowHandle; fn create_surface(..); fn resize(..); fn clear(..); }` | Contract for a render backend: create a surface for a window, resize it, and clear it to a color. |
| `hal::Shell`, `shell::Shell` | `trait Shell: Sized { type Error; type Waker; type Window; fn waker(&self); fn wake(waker: &Self::Waker); fn create_window(..); fn create_window_logical(..); fn create_window_with(..); fn drain_events(&mut self); fn run<F>(self, frame: F); }` | Contract for a windowing backend: create windows, wake the loop from other threads, drain events, and run the frame loop. |
| `hal::RenderedGlyph`, `hal::text::RenderedGlyph` | `#[non_exhaustive] pub struct RenderedGlyph { font_id: u64, font_index: u32, glyph_id: u32, x: f32, y: f32, size: f32, color: Color, variation_coords: Box<[i16]>, skew: f32 }` | One positioned glyph that the text engine gives to the renderer. |
| `hal::TextAffinity`, `hal::HitPosition`, `hal::text::TextAffinity`, `hal::text::HitPosition` | `enum TextAffinity { Downstream, Upstream }`; `struct HitPosition { byte_index: usize, affinity: TextAffinity }` | Result of a text hit test: the byte index and the side of a line break or bidi boundary that the caret belongs to. |
| `hal::ParagraphMetrics`, `hal::TextEngine`, `hal::text::ParagraphMetrics`, `hal::text::TextEngine` | `trait ParagraphMetrics { fn width; fn height; fn line_count; fn min_content_width; fn max_content_width }`; `trait TextEngine { type Error; type Paragraph: ParagraphMetrics; fn layout_paragraph(&self, text: &str, max_width: Option<f32>) -> Result<Self::Paragraph, Self::Error>; fn clear_cache(&self) }` | Contract for a text shaper: lay out a paragraph at a width and report its size and line count. |
| `hal::ParagraphMetrics` | `trait ParagraphMetrics { fn width(&self) -> f32; fn height(&self) -> f32; fn line_count(&self) -> usize; fn min_content_width(&self) -> f32; ... }` | Reports the measured size of a laid-out paragraph. |
| `hal::TextEngine` | `trait TextEngine { type Error; type Paragraph: ParagraphMetrics; fn layout_paragraph(&self, text: &str, max_width: Option<f32>, ..) -> Result<Self::Paragraph, Self::Error>; ... }` | Shapes and lays out text into paragraphs. |

## Module `icons`

| Name | Signature | Purpose |
| --- | --- | --- |
| `icons::IconBaker` | `struct IconBaker`; `const fn new() -> Self`; `fn bake(&self, svg_source: &str, width: u32, height: u32) -> Result<BakedIcon, BakeError>` | Rasterizes an SVG icon into an alpha mask of the given size. |
| `icons::BakedIcon` | `struct BakedIcon { width: u32, height: u32, alpha: Vec<u8> }` | Holds a baked icon as one alpha byte per pixel. |
| `icons::BakeError` | `enum BakeError { SvgParseError(String), PathOverflow, TooManySegments(u32), ViewBoxMissing }` | Reports why `IconBaker::bake` failed. |
| `icons::builtin::HEART_SVG`, `icons::builtin::GEAR_SVG`, `icons::builtin::SEARCH_SVG`, `icons::builtin::ARROW_RIGHT_SVG`, `icons::builtin::X_SVG`, `icons::builtin::ARROW_UP_SVG`, `icons::builtin::ARROW_DOWN_SVG`, `icons::builtin::CHEVRONS_UP_DOWN_SVG`, `icons::builtin::CHECK_SVG` | `const *_SVG: &str` | Hold the SVG source of each bundled icon. |
| `icons::builtin::ALL` | `const ALL: [(&str, &str); 8]` | Lists every bundled icon as a `(name, svg)` pair. |

## Module `kernel`

| Name | Signature | Purpose |
| --- | --- | --- |
| `kernel::Live`, `kernel::SortDirection` | re-exports of `accesskit::Live`, `accesskit::SortDirection` | AccessKit values for live-region politeness and column sort state in accessibility props. |
| `kernel::HostMode` | `enum HostMode { Pixel, Cell }` | Tells widgets whether the host lays out in pixels (window) or in terminal cells. Default is `Pixel`. |
| `kernel::schedule_keyed_rebuild` | `fn schedule_keyed_rebuild<K: Clone + Eq + Hash + 'static, N: IntoNode + 'static>(parent: NodeId, prior: Rc<RefCell<Vec<(K, NodeId)>>>, incoming: Vec<(K, Option<N>)>)` | Queues a keyed reconcile of the children of `parent`. Children with a known key keep their node; new keys mount and missing keys unmount. Pass `None` for a key that is already mounted to keep it without building a node. |
| `kernel::FocusScopeKind` | `enum FocusScopeKind { Modal, Popup, Inert }` | Kind of focus scope on a node: a modal traps focus, a popup closes on focus loss, and inert content takes no focus. |
| `kernel::TextCursor` | `struct TextCursor { caret: Signal<usize>, anchor: Signal<Option<usize>> }` | Caret and selection anchor of an editable text node, exported to accessibility. |
| `kernel::Node` | `struct Node` | A node in the tree, as `NodeBuilder::mutate` sees it. |
| `kernel::NodeType` | `enum NodeType` | Container, text, image, or fragment. |
| `kernel::NodeExtensions` | `struct NodeExtensions` | Typed data attached to a node. |
| `kernel::TextBuilder` | `struct TextBuilder`; `wrap()`, `ellipsize()`, `nowrap()`, `runs(Vec<TextRun>)`, `label(name)`, `description(text)`, `role(Role)`, `keyboard_shortcut(shortcut)`, `focusable(bool)`, `tab_index(i32)`, `hittable(bool)`, `mutate(FnOnce(&mut Node))`, `add(&mut Cx) -> NodeId` | Builder of a text node. `wrap` breaks lines at the width; `ellipsize` keeps one line and ends cut text with an ellipsis. `runs` styles ranges of the text. `label` replaces the text that screen readers read. |
| `kernel::ImageBuilder` | `alt`, `fit`, `crop`, `mutate`, plus `WidgetLayoutExt` | The one image builder. `image`, `image_rgba`, and `image_bytes` return it. |
| `kernel::ImageCrop` | `struct ImageCrop { top: u32, height: u32 }` | Vertical crop of an image: rows `top..top + height`, clamped to the image. A crop with no rows inside the image draws nothing. The fit applies to the cropped rows. |
| `kernel::ImageRaster` | `struct ImageRaster` (opaque, `Clone + PartialEq + Debug`) | Resolution-independent source of an image. Paths and SVG icons carry one; the renderer asks it for pixels at the physical destination size. |
| `kernel::ObjectFit` | `enum ObjectFit { Fill, Contain, Cover }` | How an image fills its box. |
| `kernel::TopLayer` | `struct TopLayer` | Marker of a node that paints above the tree. |
| `kernel::A11yProps` | `struct A11yProps`; `required: Option<Signal<bool>>`, `position_in_set: Option<usize>`, `size_of_set: Option<usize>`, .. | Accessibility properties of a node. `virtual_list` sets each row's position and the item count, so a screen reader reads "5 of 200". |
| `kernel::AccessibleName` | `enum AccessibleName` | None, static, or signal name. |
| `kernel::Surface` | `struct Surface { key: u64 }` | Marker of a host-drawn surface. |

## Module `layout`

| Name | Signature | Purpose |
| --- | --- | --- |
| `layout::Display`, `layout::Position`, `layout::FlexDirection`, `layout::BoxSizing` | `enum Display { Block, Flex, Grid, None }`; `enum Position { Relative, Absolute }`; `enum FlexDirection { Row, Column, RowReverse, ColumnReverse }`; `enum BoxSizing { ContentBox, BorderBox }` | Set the display mode, positioning, flex direction, and box sizing of a `StyleData` node. |
| `layout::JustifyContent`, `layout::AlignContent`, `layout::AlignItems`, `layout::AlignSelf` | `enum JustifyContent` and `enum AlignContent { Start, End, Center, Stretch, SpaceBetween, SpaceAround, SpaceEvenly }`; `enum AlignItems { Start, End, Center, Baseline, Stretch }`; `type AlignSelf = AlignItems` | Set main-axis and cross-axis alignment of a `StyleData` node and its children. |
| `layout::LengthPercentage`, `layout::LengthPercentageAuto` | `enum LengthPercentage { Px(f32), Percent(f32), Text(TextLength) }`; `enum LengthPercentageAuto { Px(f32), Percent(f32), Auto }` | Padding, margin, gap, and inset values of a `StyleData` node. |
| `layout::TextGeometry`, `layout::text_geometry::TextGeometry` | `trait TextGeometry { fn hit(..) -> HitPosition; fn caret(..) -> CaretGeometry; fn selection_rects(..) -> Vec<Rect> }` | Maps between points and text positions for hit testing, carets, and selection. |
| `layout::CaretGeometry`, `layout::text_geometry::CaretGeometry` | `struct CaretGeometry { x: f32, top: f32, height: f32 }` | Holds the position and height of a text caret. |
| `layout::ReactivePosition` | `struct ReactivePosition` | Extension that `position_reactive` attaches. |
| `layout::BoundsReactive` | `struct BoundsReactive` | Extension that `bounds_reactive` attaches. |
| `layout::FlexDirection` | `enum FlexDirection { Row, Column, RowReverse, ColumnReverse }` | Main axis of a flex box. |
| `layout::AlignSelf` | `type AlignSelf = AlignItems` | Cross-axis alignment of one child. |
| `layout::Position` | `enum Position { Relative, Absolute }` | Positioning mode. |
| `layout::Overflow` | `enum Overflow { Visible, Hidden, Scroll }` | Overflow mode. |
| `layout::LengthPercentageAuto` | `enum LengthPercentageAuto { Px, Percent, Auto }` | A px, percent, or auto length. |
| `layout::TrackSize` | `enum TrackSize { Px(f32), Fr(f32), Auto, MinMax(f32, f32) }` | Size of a grid track. |
| `layout::TextLength` | `enum TextLength { Rows(f32), Cols(f32), Em(f32) }` | A length relative to the text metrics. |
| `layout::LayoutEngine` | `struct LayoutEngine`; `take_republished(&mut self) -> bool` | The layout pass. `Host` owns one. `take_republished` reports whether the last pass changed a size or bounds binding, and resets the flag. `Host` runs up to four passes per frame until no binding changes. |
| `layout::LayoutResult` | `struct LayoutResult` | The laid-out rectangle of a node. |
| `layout::LayoutError` | `enum LayoutError` | The layout pass failed. |
| `layout::TextMeasure` | `trait TextMeasure` | Text measurement that the layout pass calls. `font_generation` changes when the loaded fonts change; layout drops cached text sizes when it does. |

## Module `profile`

| Name | Signature | Purpose |
| --- | --- | --- |
| `profile::Category` | `enum Category { Layout, Paint, Render, A11y, Reactive, App }` (`name()`) | Groups profile phases by subsystem for reports and the frame HUD. |
| `profile::Phase` | `enum Phase { Effect, Layout, LayoutSolve, ..., Paint, RenderCpu, A11y, TuiPaint, TuiRaster, TuiWrite, TuiPresent, Gpu, GpuWait, RenderPlan, RenderEncode, RenderRetain, RenderSubmit }` (`name()`, `category()`) | Names one timed stage of a frame. `category()` returns its `Category`. `Gpu` is GPU execution time from timestamp queries; `GpuWait` is time a headless host blocks on the GPU. The `Render*` phases split renderer CPU time into the damage plan, command encoding, the retained scene copy, and the queue submit. |
| `profile::Counter`, `profile::Cause`, `profile::BackendKind`, `profile::FallbackCause` | `#[non_exhaustive] enum` with `name() -> &'static str` | Name a per-frame counter, the event that caused a frame, the active renderer backend, and why the host fell back from the GPU backend. |
| `profile::BackendStatus` | `struct BackendStatus { kind: BackendKind, fallback: FallbackCause }`; `const TERMINAL` | Records which backend renders the frame and the fallback cause, if any. |
| `profile::PhaseTime`, `profile::CounterValue` | `type PhaseTime = (Phase, u64)`; `type CounterValue = (Counter, u64)` | Pair a phase with its time in nanoseconds, and a counter with its value. |
| `profile::Span` | `struct Span` (drop guard) | Measures one phase from creation until drop. Create it with `profile::span`. |
| `profile::Frame` | `struct Frame { index: u32, cause: Cause, wall_ns: u64, end_us: u64, phases: Vec<PhaseTime>, counts: Vec<CounterValue>, hot: Vec<Attribution>, gpu_passes: Vec<(&'static str, u64)>, backend: Option<BackendStatus> }`; `counter`, `total_ns`, `hottest` | Holds the recorded timings, counters, attributions, and GPU pass times of one frame. |
| `profile::FRAME_BUDGET_NS`, `profile::INTERACT_BUDGET_NS` | `const FRAME_BUDGET_NS: u64 = 16_670_000`; `const INTERACT_BUDGET_NS: u64 = 200_000_000` | The 60 Hz frame budget and the input-to-frame latency budget. Reports compare frames against `frame_budget_ns`. |
| `profile::frame_budget_ns`, `profile::BUDGET_ENV`, `profile::DEFAULT_BUDGET_MS` | `fn frame_budget_ns() -> u64`; `const BUDGET_ENV: &str = "SIGIL_PROFILE_BUDGET_MS"`; `const DEFAULT_BUDGET_MS: f64 = 8.33` | The frame budget that reports use: `SIGIL_PROFILE_BUDGET_MS` in milliseconds, or 8.33 ms (120 Hz). It is read once. |
| `profile::Hotspot`, `profile::Attribution` | `#[non_exhaustive] enum Hotspot { LayoutDirtyKernel, LayoutDirtySeed, LayoutResized, LayoutRoot, EmitResolved, GpuLayer, GpuFullRepaint, RowRebuild }` (`name()`); `struct Attribution { hotspot: Hotspot, subject: String, count: u64 }` | Name what caused a cost and who caused it. The summary's `why` section lists the top subjects per hotspot. |
| `profile::attribute`, `profile::MAX_SUBJECTS`, `profile::OTHER_SUBJECT` | `fn attribute(hotspot: Hotspot, n: u64, subject: impl FnOnce() -> String)`; `const MAX_SUBJECTS: usize = 64`; `const OTHER_SUBJECT: &str = "(other)"` | Add `n` to `subject` under `hotspot` in the current frame. `subject` runs only while profiling is on. Past `MAX_SUBJECTS` distinct subjects per hotspot, new ones count under `OTHER_SUBJECT`. |
| `profile::gpu_pass` | `fn gpu_pass(label: &'static str, ns: u64)` | Add GPU execution time for one render pass to the current frame and to `Phase::Gpu`. The renderer calls it from timestamp queries. |
| `profile::enable`, `profile::disable`, `profile::enabled`, `profile::reset` | `fn enable()`; `fn disable()`; `fn enabled() -> bool`; `fn reset()` | Turn frame recording on or off, query it, and clear recorded frames. |
| `profile::begin_frame`, `profile::tick_frame` | `fn begin_frame(backend: BackendStatus)`; `fn tick_frame()` | Open a new recorded frame with its backend status, and close the current frame. Hosts call these once per frame. |
| `profile::span` | `fn span(phase: Phase) -> Span` | Starts timing `phase` in the current frame. |
| `profile::count`, `profile::count_add`, `profile::gauge` | `fn count(counter: Counter)`; `fn count_add(counter: Counter, n: u64)`; `fn gauge(counter: Counter, value: u64)` | Add one or `n` to a counter, or set it to a value, in the current frame. |
| `profile::interact` | `fn interact(cause: Cause)` | Marks the next frame as caused by `cause` so the report can measure input latency. |
| `profile::note_input` | `fn note_input(event: &ShellEvent)` | Marks the current frame with the cause that matches an input event: pointer press, hover, key, scroll, IME, or resize. Hosts and the test harness call it for each routed event. The summary breaks each cause down by phase. |
| `profile::dump`, `profile::dump_if_enabled` | `fn dump(path: impl AsRef<Path>) -> io::Result<()>`; `fn dump_if_enabled()` | Write recorded frames as a Chrome trace file and print a summary. `dump_if_enabled` writes to the path set by the environment, if any. |
| `profile::frames`, `profile::summary` | `fn frames() -> Vec<Frame>`; `fn summary() -> String` | Return the recorded frames, and a text summary of them. |

## Module `reactive`

| Name | Signature | Purpose |
| --- | --- | --- |
| `reactive::Subscription` | `#[must_use] pub struct Subscription` | Handle returned by `EventChannel::subscribe`. Dropping it removes the subscriber. |
| `reactive::effect_named` | `fn effect_named<F: FnMut() + 'static>(name: &'static str, f: F)` | Same as `effect`, with a name that shows in diagnostics. |
| `reactive::Owner` | `struct Owner`; `fn new(name: &'static str, parent: Option<OwnerId>) -> Self` | Reactive scope that owns effects, signals, and cleanups. Disposing it disposes its children. |
| `reactive::DrawPhase` | `enum DrawPhase { Idle, Effect, Layout, Paint }` | Phase of the frame that the runtime is in. Signal writes during paint are rejected. |
| `reactive::on_cleanup` | `fn on_cleanup(f: impl FnOnce() + 'static)` | Run `f` when the current owner is disposed. |
| `reactive::batch` | `fn batch<R>(f: impl FnOnce() -> R) -> R` | Group writes into one update. |
| `reactive::untrack` | `fn untrack<R>(f: impl FnOnce() -> R) -> R` | Run `f` without subscribing the current effect to its reads. |
| `reactive::defer` | `fn defer(f: impl FnOnce() + 'static)` | Run `f` after the current effect returns, or now outside an effect. An effect uses it to consume the signal that triggers it. |
| `reactive::EventChannel` | `struct EventChannel<T>`; `emit`, `subscribe` | Send events between parts of the app. |
| `reactive::StyleReactiveEffect` | `struct StyleReactiveEffect`; `new()` | Holds a reactive atom channel that updates a node's style when its value changes. |
| `reactive::Runtime` | `struct Runtime`; `Runtime::scope(name, f) -> R` | Run code inside a reactive runtime. |

## Module `renderer`

| Name | Signature | Purpose |
| --- | --- | --- |
| `renderer::WgpuRenderer` | `struct WgpuRenderer; fn new() -> Result<Self, RendererError>; fn render_command_scene(&mut self, surface: &mut WgpuSurface, scene: &CommandScene, clear_color: Color, viewport: (f32, f32)) -> Result<(), RendererError>; fn render_command_scene_offscreen(&mut self, target: OffscreenTarget<'_>, scene: &CommandScene, clear_color: Color) -> Result<(), RendererError>; fn device/queue/instance` | The GPU renderer. It draws a `CommandScene` to a window surface or an offscreen texture and rebuilds the device after device loss. |
| `renderer::WgpuRenderer::drain_gpu_timings` | `fn drain_gpu_timings(&mut self)` | Report finished GPU timestamp results to the profiler. A headless caller calls it after a blocking `device.poll` to get GPU pass times in the same frame. |
| `renderer::WgpuSurface` | `struct WgpuSurface; const fn size(&self) -> PhysicalSize; const fn format(&self) -> wgpu::TextureFormat` | A configured GPU window surface that `WgpuRenderer` presents to. |
| `renderer::CpuRenderer`, `renderer::CpuSurface` | `struct CpuRenderer; fn new() -> Self; fn render(&mut self, scene: &CommandScene, clear: Color, viewport: (f32, f32), size: PhysicalSize) -> Result<Pixmap, RendererError>; fn render_command_scene(&mut self, surface: &mut CpuSurface, scene, clear, viewport) -> Result<(), RendererError>`; `CpuSurface::size(&self) -> PhysicalSize` | The software renderer (feature `cpu`, selected by `SIGIL_BACKEND=cpu`). It draws the same `CommandScene` to a pixmap or a window buffer. |
| `renderer::OffscreenTarget` | `struct OffscreenTarget<'a> { device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView, format: wgpu::TextureFormat, viewport: (f32, f32), scale: f32 }` | A caller-owned texture target for `WgpuRenderer::render_command_scene_offscreen`. |
| `renderer::ScreenSubmission` | `struct ScreenSubmission<'a> { device, queue, view, format, viewport: (f32, f32), scale: f32, clear: Option<Color> }` | Target for drawing a scene in screen space into an embedder's render pass. |
| `renderer::RendererError` | `#[non_exhaustive] enum RendererError { NoAdapter, DeviceRequest(..), SurfaceCreate(..), NoSupportedFormat, SurfaceValidation, NotInitialized, InvalidViewport, Layer(LayerError), Software(String) }` | Errors from device setup, surface creation, frame validation, offscreen layers, and the software renderer. |
| `renderer::LayerError` | `enum LayerError { Empty, TooLarge { requested: u32, max: u32 } }` | Why the renderer could not allocate an offscreen layer: the extent is empty or larger than the device limit. `RendererError::Layer` carries it. |
| `renderer::WindowHandle`, `renderer::into_window_handle` | `struct WindowHandle`; `fn into_window_handle<W>(window: W) -> WindowHandle where W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static` | Wrap a platform window so the renderer can create a surface for it. |

### `renderer::capture`

| Name | Signature | Purpose |
| --- | --- | --- |
| `renderer::capture::FORMAT`, `renderer::capture::CaptureExtent` | `const FORMAT: wgpu::TextureFormat = Rgba8UnormSrgb`; `struct CaptureExtent { width: u32, height: u32 }` | The texture format and size of a capture target (feature `scene`). |
| `renderer::capture::capture_texture`, `renderer::capture::read_texture` | `fn capture_texture(device: &wgpu::Device, extent: CaptureExtent) -> wgpu::Texture`; `fn read_texture(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, extent: CaptureExtent) -> Result<Vec<u8>, Box<dyn Error>>` | Create a render target texture that supports copy-out, and read it back as tightly packed RGBA8 rows. |

## Module `scene`

| Name | Signature | Purpose |
| --- | --- | --- |
| `scene::SceneCommand` | `enum SceneCommand { PushClip(Rect), PopClip, PushTransform(Transform), PopTransform, SaveLayer { opacity }, RestoreLayer, PushLayer(LayerScope), PopLayer, Rect(DrawRect), Gradient(DrawGradient), Pattern(DrawPattern), DrawShadow { rect, radii, shadow }, GlyphRun(DrawGlyphRun), Sprite(DrawSprite), Image(DrawImage), Decoration(DrawDecoration) }` | One draw or state command in a `CommandScene`. Both hosts consume the same command list. |
| `scene::emit_scene`, `scene::emit_scene_clipped` | `fn emit_scene(cx: &Cx, root: NodeId) -> CommandScene`; `fn emit_scene_clipped(cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene` | Walk the laid-out tree from `root` and return its draw commands. The clipped form skips nodes outside `clip`. |
| `scene::paint_backdrop` | `fn paint_backdrop(scene: &mut CommandScene, theme: &Theme, viewport: (f32, f32))` | Put a rectangle of the theme's `Token::Surface` colour under every command in `scene`, as an opaque window paints behind its tree. |
| `scene::DrawRect`, `scene::CellRect` | `struct DrawRect { node, rect, color, border_radius, opacity, stroke, tone, leaf, border, outline }`; `struct CellRect { tone, leaf, border, outline }` | A filled rectangle command. `CellRect` holds the cell-grid hints (tone, leaf, border, outline) that the terminal host reads. |
| `scene::DrawGradient`, `scene::LinearGradient`, `scene::GradientStop` | `struct DrawGradient { node, rect, gradient, border_radius, opacity, stroke, .. }`; `LinearGradient::new(start: Point, end: Point, stops)`; `GradientStop::new(offset: f32, color: Rgba8)` | A rectangle filled with a linear gradient between two points through ordered colour stops. |
| `scene::DrawPattern`, `scene::PatternKind` | `struct DrawPattern { node, rect, pattern, border_radius, opacity, stroke }`; `enum PatternKind { Slash { color, period, line_width }, Checker { color_a, color_b, cell } }` | A rectangle filled with a repeating slash or checker pattern. |
| `scene::PatternKind::normalized`, `scene::PatternKind::tile_extent` | `fn normalized(self) -> PatternKind`; `fn tile_extent(self) -> f32` | The pattern both renderers paint: `period` and `cell` clamped to 1..=1024 logical pixels (NaN becomes 1), `line_width` clamped to 0..=period (NaN becomes 0). Tiles are anchored at the top-left corner of the drawn rect, in logical pixels, independent of device scale. Colours are straight sRGB and each cell keeps its own alpha. `tile_extent` is one repeat: `period` for a slash, `2 * cell` for a checker. |
| `scene::LinearGradient::normalized` | `fn normalized(&self) -> LinearGradient` | The gradient both renderers paint: stops sorted by offset, offsets clamped to 0..=1 with the colour at each clamp point kept, and a zero-length line or no stops reduced to one solid stop (the last stop's colour, or transparent). |
| `scene::PaintFill` | `enum PaintFill { Solid(Rgba8), LinearGradient(LinearGradient), Pattern(PatternKind) }` | The fill of a painted area: solid colour, gradient, or pattern. |
| `scene::DrawImage` | `struct DrawImage { node, source: ImageSource, fit: ObjectFit, destination: Rect, opacity: u16, crop: Option<ImageCrop>, raster: Option<ImageRaster> }` | An image command fitted into a destination rectangle, with optional crop. With a `raster` and no crop, both renderers draw pixels made at the physical destination size, snapped to the device pixel grid; `source` is the fallback. |
| `scene::DrawImage::source_at` | `fn source_at(&self, scale: f32) -> (ImageSource, Rect)` | The image and destination to paint at `scale`. With a `raster` and no crop, the pixels are made at the device size (each side at most 4096) and the rectangle snaps to the device pixel grid; otherwise `source` and `destination`. Embedders that paint `DrawImage` call this. |
| `scene::DrawSprite`, `scene::SpriteSource`, `scene::SpriteKind` | `DrawSprite::new(style: &SceneSpriteStyle)`; `struct SpriteSource { atlas: u32, region: Rect, phase: f32 }`; `enum SpriteKind { Mono, Subpixel, Poly }` | A region of a glyph or icon atlas drawn into a destination rectangle. |
| `scene::DrawDecoration`, `scene::DecorationKind` | `struct DrawDecoration { node, kind, band, color, opacity, period, line_width }`; `enum DecorationKind { Underline, Strikethrough, WavyUnderline }` | A text decoration line drawn across a band. |
| `scene::DecorationGeometry`, `scene::DrawDecoration::geometry` | `struct DecorationGeometry { band, center, thickness, amplitude, period }`; `fn geometry(&self) -> Option<DecorationGeometry>`; `fn wave_y(&self, x) -> f32`, `fn line(&self) -> Rect`, `fn reach(&self) -> f32` | Resolved decoration shape that both renderers draw, in the band's local space before the transform. Ink stays inside the band: straight lines clamp `line_width` to the band height; wavy lines use a stroke of 0.8 x `line_width` and an amplitude of 0.75 x `line_width`, both shrunk to fit the band, with the period raised to at least 4 x (amplitude + stroke) and 1. The wave is a sine that starts at the band centre at the band's left edge. Returns `None` for a non-finite or empty band and for a non-finite, zero or negative `line_width`. |
| `scene::DrawGlyphRun`, `scene::shape_glyph_run` | `fn shape_glyph_run(rect: Rect, text: String, color: Rgba8, style: TextStyle, opacity: u16) -> GlyphRun` (feature `scene`) | `DrawGlyphRun` is a shaped text command. `shape_glyph_run` shapes text into a `GlyphRun` ready for `SceneBuilder::draw_glyph_run`. |
| `scene::SceneRectStyle`, `scene::SceneGradientStyle`, `scene::ScenePatternStyle` | `SceneRectStyle::new(color, border_radius, opacity)`, `::solid(color)`, `::rounded(color, radius)`, `.with_stroke(stroke)`; `SceneGradientStyle::new(gradient, border_radius, opacity)`; `ScenePatternStyle::new(pattern, border_radius, opacity)` | Style arguments for `SceneBuilder::draw_rect`, `draw_gradient`, and `draw_pattern`. |
| `scene::SceneSpriteStyle`, `scene::SceneDecorationStyle` | `struct SceneSpriteStyle { source, kind, destination, color, opacity }`; `struct SceneDecorationStyle { kind, band, color, opacity, period, line_width }` | Style arguments for `SceneBuilder::draw_sprite` and `draw_decoration`. |
| `scene::Stroke`, `scene::StrokeStyle`, `scene::StrokeAlign` | `struct Stroke { color: Rgba8, width: f32, style: StrokeStyle, align: StrokeAlign }`; `enum StrokeStyle { Solid, Dashed(f32) }`; `enum StrokeAlign { Centered, Inside, Outside }` | An outline on a rectangle, gradient, or pattern, with dash and alignment. A width that is not finite and positive draws nothing. `Inside` and `Centered` strokes reach at most half the shorter side inward. `Dashed(gap)` draws dashes of `max(3 * width, 1)` separated by `gap`; a gap below 1/16 or not finite draws a solid stroke. |
| `scene::LayerScope`, `scene::LayerEffect` | `struct LayerScope { clip: Rect, clip_radii: CornerRadii, effect: LayerEffect, opacity: f32 }`; `enum LayerEffect { Identity, Transform(Transform), LayerBlur { sigma } }` | A composited layer: its clip, opacity, and a transform or blur effect. Used with `SceneBuilder::push_layer`. |
| `scene::CornerRadii` | `struct CornerRadii(pub [f32; 4])`; `ZERO`, `new(tl, tr, br, bl)`, `uniform(r)`, `top(r)`, `bottom(r)`, `left(r)`, `right(r)`, `normalized_for(w, h) -> [f32; 4]` | Per-corner radii in the order top-left, top-right, bottom-right, bottom-left. `normalized_for` clamps negative and NaN radii to 0 and scales them down proportionally, as CSS does, so adjacent radii fit each side. Both renderers use it. |
| `scene::Shadow`, `scene::ShadowKind` | `Shadow::new(offset: [f32; 2], blur: f32, spread: f32, color: [f32; 4])`, `Shadow::glow(color: Rgba8, spread, blur)`, `.with_kind(kind)`; `enum ShadowKind { Outer, Inner, Glow }` | A box shadow with linear colour, offset, blur, and spread. `glow` builds a centred glow from an sRGB colour. |

## Module `shell`

| Name | Signature | Purpose |
| --- | --- | --- |
| `shell::WinitShell` | `struct WinitShell` (implements `hal::Shell`) | Window host backed by winit. |
| `shell::WinitWaker`, `shell::WakeEvent` | `struct WinitWaker`; `fn wake(&self) -> Result<(), ShellError>`; `struct WakeEvent;` | Wakes the winit event loop from any thread. Repeated wakes before the loop runs merge into one `WakeEvent`. |
| `shell::WinitWindowHandle` | `struct WinitWindowHandle(u64)`; `const fn id(self) -> u64` | Id of a window that `WinitShell` created. |
| `shell::ShellError` | `enum ShellError { EventLoop(EventLoopError), EventLoopClosed, WakerPoisoned, AlreadyRun, Window(String) }` | Errors from `WinitShell` and `WinitWaker`. |

## Module `style`

| Name | Signature | Purpose |
| --- | --- | --- |
| `style::Transform2D` | `struct Transform2D`; `translate(x, y)`, `scale(x, y)`, `rotate(radians)`, `origin(x, y)`, `IDENTITY` | Holds a 2D translation, scale, and rotation for a node. The renderer applies it around the transform origin. |
| `style::SELECTION_FILL` | `const SELECTION_FILL: Rgba8` | Gives the fill color for selected text. |
| `style::SPACING_STEP` | `const SPACING_STEP: f32 = 4.0` | Sets the pixel size of one spacing step in style atoms. |
| `style::GradientAxis`, `style::GradientStopKey`, `style::BgGradient` | `enum GradientAxis { TopToBottom, BottomToTop, LeftToRight, .. }`; `struct GradientStopKey { offset: u16, color: ColorAtom }`; `struct BgGradient { axis: GradientAxis, stops: &'static [GradientStopKey] }`; `BgGradient::owned(axis, Vec<GradientStopKey>)` | Describe a linear background gradient: its direction and its color stops. `owned` takes stops built at run time; equal stop lists share one copy for the life of the process. |
| `style::DisplayKind`, `style::FlexDir`, `style::AlignKind`, `style::JustifyKind`, `style::FlexWrapKind`, `style::OverflowKind` | `enum DisplayKind { Flex, Block, Grid, .. }`; `enum FlexDir { Row, Column, RowReverse, .. }`; `enum AlignKind`; `enum JustifyKind`; `enum FlexWrapKind { NoWrap, Wrap }`; `enum OverflowKind { Visible, Hidden, Scroll, .. }` | Typed layout values that style atoms carry for display, flex direction, alignment, justification, wrap, and overflow. |
| `style::TransitionId`, `style::MillisU16` | `struct TransitionId(pub u16)`; `struct MillisU16(pub u16)` | Identify a style transition and give its duration in milliseconds. |
| `style::Elevation` | `enum Elevation { Sm, Base, Md, .. }` | Selects a shadow depth from the theme. |
| `style::RenderStyle` | `struct RenderStyle { background_color, background_gradient, text_color, border_radius, border_width, border_top, .., border_color, shadow, opacity: u16, transform: Transform2D }` | Holds the resolved paint properties of one node. Opacity is in units of 1/10000. |
| `style::TextDecoration` | `enum TextDecoration { None, Underline, .. }` | Selects the line drawn with text. |
| `style::binding::ReactiveStateSlot`, `style::binding::ReactiveStyleAtoms` | `type ReactiveStateSlot = Rc<Cell<StateSlot>>`; `type ReactiveStyleAtoms = Rc<Cell<&'static [StyleAtom]>>` | Shared cells that let code change a node's state slot or style atoms without rebuilding the tree. |
| `style::engine::StyleEngine` | `struct StyleEngine<'theme>`; `apply(&self, &[StyleAtom], &mut RenderStyle)`, `apply_text(&self, &[StyleAtom], &mut TextStyle)`, `resolve_color(&self, ColorAtom) -> Rgba8` | Applies style atoms to paint and text styles against a theme and tone. |
| `style::unknown_class_warning_count` | `fn unknown_class_warning_count() -> usize` | Number of unknown class names seen. |
| `style::metrics::BORDER_WIDTH`, `style::metrics::DISABLED_OPACITY`, `style::metrics::ICON_SIZE`, `style::metrics::OVERLAY_SHADOW` | `const BORDER_WIDTH: Px = Px(1)`; `const DISABLED_OPACITY: u16 = 5_000`; `const ICON_SIZE: Px = Px(20)`; `const OVERLAY_SHADOW: Elevation = Elevation::Md` | Default border width, disabled opacity, icon size, and overlay shadow for widgets. |
| `style::metrics::space` | `const fn space(steps: u8) -> f32` | Converts a number of spacing steps to pixels. |
| `style::overlay::StyleOverlay` | `struct StyleOverlay`; `new()`, `is_empty()`, `rest()`, `hover()`, `focus()`, `pressed()`, `disabled()`, `atoms_for(StateSlot) -> &[StyleAtom]` | Holds extra style atoms for each interaction state. |
| `style::resolve::render_style_for` | `fn render_style_for(id: NodeId, node: &Node, inherited: &Theme, inherited_text: Option<Rgba8>) -> RenderStyle` | Resolves the paint style of a node from its style binding and theme. |
| `style::resolve::text_style_for` | `fn text_style_for(node: &Node, inherited: &Theme, base: &TextStyle) -> TextStyle` | Resolves the text style of a node on top of a base text style. |
| `style::scale::slate_light`, `style::scale::iris_light`, `style::scale::blue_light`, `style::scale::green_light`, `style::scale::amber_light`, `style::scale::ruby_light`, `style::scale::cyan_light`, `style::scale::slate_dark`, `style::scale::iris_dark`, `style::scale::blue_dark`, `style::scale::green_dark`, `style::scale::amber_dark`, `style::scale::ruby_dark`, `style::scale::cyan_dark` | `const fn slate_light() -> ColorScale` (same shape for each) | Return the built-in 12-step color scales for light and dark themes. |
| `style::state::StateSlot` | `enum StateSlot { Rest, Focused, Hovered, Pressed, Disabled }` | Names the interaction state that selects state style atoms. |
| `style::styled::StateStyleBuilder` | `struct StateStyleBuilder` | Builds the paint for one interaction state. Implements `Paint`. |
| `style::text_style::TextLineHeight` | `enum TextLineHeight { FontSizeRelative(f32), Absolute(f32) }` | Gives line height as a multiple of font size or in pixels. |
| `style::text_style::TextStyle` | `struct TextStyle { font_family: String, font_size: f32, font_weight: u16, italic: bool, line_height: TextLineHeight, letter_spacing_em: f32, tabular_nums: bool, decoration: TextDecoration, runs: Option<TextRuns> }` | Holds the resolved typography of a text node. |
| `style::text_style::TextFace`, `style::text_style::TextRun`, `style::text_style::TextRuns` | `struct TextFace { bold, italic, mono, .. }`; `struct TextRun { start: usize, end: usize, face: TextFace, .. }`; `struct TextRuns(pub Arc<[TextRun]>)` | Mark byte ranges of a text with bold, italic, or monospace faces. |
| `style::text_style::is_typography_atom` | `const fn is_typography_atom(atom: StyleAtom) -> bool` | Returns true when a style atom changes text layout. |
| `style::text_style::StyledTextEngine` | `trait StyledTextEngine: TextEngine { fn layout_paragraph_styled(&self, text: &str, max_width: Option<f32>, style: &TextStyle) -> Result<Self::Paragraph, Self::Error>; }` | Lays out a paragraph with a text style. |
| `style::theme::ThemeBuilder` | `struct ThemeBuilder`; `new()`, `tokens(&phf::Map<&str, ThemeValue>)`, `token(&str, ThemeValue)`, `build() -> Theme` | Builds a theme from named token values. |
| `style::tone::ToneContext` | `struct ToneContext { tone: Tone }`; `new(Tone)` | Carries the tone that descendants resolve tone colors against. |
| `style::variant::VariantTokens` | `struct VariantTokens { radius: u16, shadows: bool, hover_lift: bool, font: FontFamilyHint }` | Holds the radius, shadow, hover, and font settings of a theme variant. |
| `style::variant_kit::SOLID_VARIANT`, `style::variant_kit::SOFT_VARIANT`, `style::variant_kit::GHOST_VARIANT` | `const SOLID_VARIANT: StyleVariant` (same for soft and ghost) | Built-in solid, soft, and ghost style variants with their per-state atoms. |
| `style::Px` | `struct Px(pub i16)` | A pixel length in a style atom. |
| `style::ColorOrToken` | `enum ColorOrToken { Rgba(Rgba8), Token(ColorKey), Atom(ColorAtom) }` | Colour argument of the `Paint` methods. Converts from `Token`, `Rgba8`, and `ColorAtom`. |
| `style::ColorKey` | `enum ColorKey { Token(Token), Custom(&'static str) }`, `ColorKey::parse(name)` | Typed key of a theme colour. `parse` maps a built-in name to `Token` and any other name to `Custom`. |
| `style::TokenTable` | `struct TokenTable { overrides: TokenLayer, theme: TokenLayer }` | The runtime override and theme layers of `Theme::tokens`. |
| `style::TokenLayer` | `color(ColorKey)`, `value(&str)`, `insert(&str, ThemeValue)`, `names()` | One layer: a fixed array per `Token` plus a map for custom names. |
| `style::ThemeValue` | `enum ThemeValue { Color(Rgba8), Px(f32), Ms(u16) }` | One theme token value. |
| `style::StyleState` | `struct StyleState { disabled, hovered, pressed, focused: bool }` | Interaction state for a style variant. |
| `style::StateStyle` | `StateStyle::new(rest, focused, hovered, pressed, disabled)`; `with_rest(atoms)`, `with_focused(atoms)`, `with_hovered(atoms)`, `with_pressed(atoms)`, `with_disabled(atoms)` | Atoms per interaction state. Each `with_*` replaces the atoms of one state. |
| `style::StyleVariant` | `StyleVariant::new(base, states)` | Base atoms plus `StateStyle`. |
| `style::NumericAtomKind` | `enum NumericAtomKind` | A numeric style property that `animate` drives. |

## Module `tasks`

| Name | Signature | Purpose |
| --- | --- | --- |
| `tasks::installed` | `fn installed() -> bool` | Tells whether a task dispatcher is installed on this thread. |
| `tasks::on_message` | `fn on_message<T: Send + 'static, F: Fn(T) + 'static>(rx: impl MessageReceiver<T>, f: F)` | Run `f` on the UI thread for each message, in the current owner. `rx` is a tokio `UnboundedReceiver<T>` or a bounded `Receiver<T>`. |
| `tasks::on_message_or_close` | `fn on_message_or_close<T: Send + 'static, F: Fn(T) + 'static, C: FnOnce() + 'static>(rx: impl MessageReceiver<T>, f: F, on_close: C)` | Like `on_message`. Runs `on_close` on the UI thread after the last message, when every sender is dropped. |
| `tasks::OwnerHandle` | `OwnerHandle::current() -> Option<OwnerHandle>`, `OwnerHandle::from_id(OwnerId)`, `.is_alive() -> bool`, `.run(f) -> bool` | `Send` handle of a reactive owner. `run` calls `f` inside the owner, so signals and effects it creates belong there, and returns `false` without calling `f` once the owner is disposed. |

## Module `terminal`

| Name | Signature | Purpose |
| --- | --- | --- |
| `terminal::Caps` | `struct Caps { color, graphics: Option<Protocol>, mux: Option<Mux>, keyboard, hyperlinks, underline_styles, borders, glyphs: Glyphs, ambiguous: Ambiguous, erase: Erase, cursor, sync, clipboard, backdrop, notices: Notices }`; `Caps::detect(env)`, `Caps::probe()`, `.host() -> HostCaps` | Terminal capabilities. `detect` reads environment variables; `probe` also queries the terminal. When the environment leaves the colour depth below truecolor, `probe` asks the terminal (DECRQSS) and trusts a kitty keyboard reply. When the environment names no terminal, as behind ssh or a multiplexer, `probe` asks for its version (XTVERSION); any answer turns hyperlinks on and a known name sets `notices`. Inside tmux or screen, only `TERM`, `COLORTERM`, and the probe identify the terminal; variables inherited from the outer terminal are ignored. The key protocol is `Legacy` unless the terminal is known or answers the probe. `erase` is `Erase::Default` for `TERM=screen*`, which does not erase with the current background. `SIGIL_COLOR=truecolor\|256\|16\|none` forces the colour depth; `SIGIL_ASCII=1` forces `Glyphs::Ascii`; `SIGIL_AMBIGUOUS_WIDE=1` sets `Ambiguous::Wide`. |
| `terminal::caps` | `fn caps() -> Option<Caps>` | The capabilities the terminal host probed when it entered raw mode, for the current thread. `None` before the terminal host starts, after it exits, and in a window. Read `Caps::backdrop` here to pick a light or dark theme; a second `Caps::probe` before `sigil::run` runs in cooked mode and leaves reply bytes on stdin. |
| `terminal::Erase` | `enum Erase { Background, Default }` | How clearing to the end of a line fills the cleared cells. `Default` (GNU screen) makes the writer send spaces for tinted row tails. |
| `terminal::ColorDepth`, `terminal::Keyboard`, `terminal::ClipRoute`, `terminal::Borders`, `terminal::Cursor` | `enum ColorDepth { Truecolor, Ansi256, Ansi16, Monochrome }`; `enum Keyboard { Legacy, Kitty }`; `enum ClipRoute { Pbcopy, Osc52 }`; `enum Borders { Rounded, Light, Ascii }`; `enum Cursor { Bar, Block }` | Capability fields of `Caps`: colour depth, key protocol, clipboard route, border glyph set, and caret shape. |
| `terminal::Backdrop` | `enum Backdrop { Dark, Light }`; `Caps::backdrop: Option<Backdrop>` | Terminal background brightness. `Caps::detect` reads it from `COLORFGBG`; `None` means unknown. With no theme file set, the terminal host uses the dark theme on `Dark`. |
| `terminal::Glyphs`, `terminal::glyphs` | `enum Glyphs { Unicode, Console, Ascii }`; `fn glyphs() -> Glyphs`; `Glyphs::glyph(self, char) -> char`; `Glyphs::text(self, &str) -> Cow<'_, str>` | Glyph set of the terminal and its one glyph map. `Ascii` for a non-UTF-8 locale, `TERM=dumb`, or `TERM=vt100`; `Console` for `TERM=linux`, which keeps box drawing and full and shade blocks and maps eighth blocks and braille to the nearest supported glyph; `Unicode` otherwise. The terminal host maps every painted cell through it. `glyphs()` reads the process environment, so apps pick glyphs through the same map. |
| `terminal::install_clipboard` | `fn install_clipboard(caps: Caps)` | Install the process clipboard for the terminal host, routed by `caps.clipboard`. |
| `terminal::notify`, `terminal::progress` | `fn notify(title: &str, body: &str) -> io::Result<()>`; `fn progress(value: Option<u8>) -> io::Result<()>` | Send a desktop notification, or set the tab progress (`Some(0..=100)`) and clear it (`None`). Each writes only what `Caps::notices` says the terminal supports, and nothing when no terminal host is live. |
| `terminal::Notices`, `terminal::Notify` | `struct Notices { pub notify: Option<Notify>, pub progress: bool }`; `enum Notify { Osc9, Osc777 }` | The notification sequence and the tab-progress sequence (OSC 9;4) a terminal supports. |
| `terminal::Mux`, `terminal::detect_mux` | `enum Mux { Tmux, Screen }`; `fn detect_mux(env: impl Fn(&str) -> Option<String>) -> Option<Mux>` | Detect a terminal multiplexer from `TMUX` or `STY`, so graphics output can be wrapped for it. |
| `terminal::Placement` | `struct Placement { col: u16, row: u16, cols: u16, rows: u16, source: ImageSource, crop: Option<(u32, u32)> }`; `.same(&other) -> bool` | An image placed on the cell grid. `same` compares position and source. |
| `terminal::blocks`, `terminal::blocks_window`, `terminal::sextants`, `terminal::sextants_window` | `fn blocks(source: &ImageSource, cols: usize, rows: usize) -> Vec<Vec<(Rgba8, Rgba8)>>`; `fn sextants(..) -> Vec<Vec<(char, Rgba8, Rgba8)>>`; `*_window(source, window: (u32, u32), cols, rows)` | Downsample an image to half-block or sextant cells for terminals without a graphics protocol. The window forms sample only a vertical pixel range. |
| `terminal::png_window`, `terminal::base64` | `fn png_window(source: &ImageSource, top: u32, height: u32) -> Vec<u8>`; `fn base64(bytes: &[u8]) -> String` | Encode a vertical slice of an image as PNG, and encode bytes as base64 for graphics escape sequences. |
| `terminal::key_down`, `terminal::map_key` | `fn key_down(key: impl Into<LogicalKey>) -> ShellEvent`; `fn map_key(event: &crossterm::event::KeyEvent) -> Option<ShellEvent>` | Build a key-press event, or translate a crossterm key event to a shell event. |
| `terminal::cell_point`, `terminal::pointer`, `terminal::wheel`, `terminal::map_mouse` | `fn cell_point(col: u16, row: u16) -> Point`; `fn pointer(phase: PointerPhase, col: u16, row: u16) -> ShellEvent`; `fn wheel(col: u16, row: u16, lines: f32) -> ShellEvent`; `fn map_mouse(event: &MouseEvent, cell: f32, last: &mut Point) -> Option<ShellEvent>` | Convert cell coordinates to layout points and build pointer and wheel events, or translate a crossterm mouse event. |
| `terminal::Cell`, `terminal::CONTINUES` | `struct Cell { text, fg: Ink, bg: Ink, bold, italic, dim, lines: Lines, underline_ink: Ink, link: Option<SmolStr> }`; `Cell::blank()`; `const CONTINUES: &str = ""` | One grid cell. A cell whose text is `CONTINUES` is the trailing half of a wide glyph. |
| `terminal::Ink`, `terminal::Underline` | `enum Ink { Bare, Of(Tone), Exact(Rgba8) }`; `enum Underline { None, Straight, Curly }` | Cell colour (unset, theme tone, or exact RGBA) and underline shape. |
| `terminal::Marks` | `Marks::collect(cx: &Cx, focus: Option<NodeId>, scene: &CommandScene, cell: f32) -> Marks` | Collect link targets and the caret node from a frame so the rasteriser can mark those cells. |
| `terminal::Paint`, `terminal::raster_into`, `terminal::raster_with` | `struct Paint<'a> { theme: &Theme, cell: f32, caps: Caps, marks: &Marks }`; `fn raster_into(grid: &mut Grid, scene: &CommandScene, theme: &Theme, cell: f32)`; `fn raster_with(grid: &mut Grid, scene: &CommandScene, paint: &Paint<'_>)` | Draw a scene into an existing grid. `raster_with` takes capabilities and marks as well as the theme. |
| `terminal::EventPump`, `terminal::AfterFrame` | `type EventPump<'a> = Box<dyn FnMut(&mut Vec<ShellEvent>) + 'a>`; `type AfterFrame<'a> = Box<dyn FnMut(&mut Host, &mut EventRouter, &Screen) + 'a>` | Callback types for `Hooks`: inject extra events before a frame, and run code after a frame is drawn. |
| `terminal::calibrate` | `fn calibrate(out: &mut impl Write) -> io::Result<()>` | Measure how the terminal renders wide glyphs and set the cell width rule to match. |
| `terminal::render` | `mod render` | Offscreen rendering for embed hosts (feature `render`). |
| `terminal::render::PhysicalSize` | `struct PhysicalSize { width: u32, height: u32 }`; `const fn new(width, height)` | Holds a surface size in physical pixels. |
| `terminal::render::RenderedGlyph` | `struct RenderedGlyph { font_id, font_index, glyph_id, x, y, size, color, variation_coords, skew }` | Describes one shaped glyph with its font, position, size, and color, ready to draw. |
| `terminal::render::CornerRadii` | `struct CornerRadii(pub [f32; 4])` | Re-export of `scene::CornerRadii`. Holds the four corner radii of a rounded rectangle. |
| `terminal::render::SceneCommand` | `enum SceneCommand { PushClip, PopClip, PushTransform, PopTransform, SaveLayer, RestoreLayer, PushLayer(LayerScope), PopLayer, Rect(DrawRect), Gradient(DrawGradient), Pattern(DrawPattern), DrawShadow { rect, radii, shadow }, GlyphRun(DrawGlyphRun), Sprite(DrawSprite), Image(DrawImage), Decoration(DrawDecoration) }` | Re-export of `scene::SceneCommand`. Lists one draw or state command in a `CommandScene`. Both hosts replay the commands in order. |
| `terminal::render::DrawRect`, `terminal::render::SceneRectStyle` | `DrawRect::new(rect: Rect, style: SceneRectStyle) -> DrawRect`; `SceneRectStyle::{new, solid, rounded, with_stroke}` | Re-export of `scene::DrawRect`, `scene::SceneRectStyle`. Draws a solid rectangle with optional corner radii, opacity, and stroke. |
| `terminal::render::DrawGradient`, `terminal::render::SceneGradientStyle` | `DrawGradient::new(rect: Rect, style: SceneGradientStyle) -> DrawGradient`; `SceneGradientStyle::{new, with_stroke}` | Re-export of `scene::DrawGradient`, `scene::SceneGradientStyle`. Draws a rectangle filled with a linear gradient. |
| `terminal::render::DrawPattern`, `terminal::render::ScenePatternStyle` | `DrawPattern::new(rect: Rect, style: ScenePatternStyle) -> DrawPattern`; `ScenePatternStyle::{new, with_stroke}` | Re-export of `scene::DrawPattern`, `scene::ScenePatternStyle`. Draws a rectangle filled with a repeating pattern. |
| `terminal::render::DrawSprite`, `terminal::render::SceneSpriteStyle` | `DrawSprite::new(style: &SceneSpriteStyle) -> DrawSprite` | Re-export of `scene::DrawSprite`, `scene::SceneSpriteStyle`. Draws a region of a sprite atlas into a destination rectangle with a tint color. |
| `terminal::render::DrawDecoration`, `terminal::render::SceneDecorationStyle` | `struct DrawDecoration { node, kind: DecorationKind, band: Rect, color, opacity, period, line_width }` | Re-export of `scene::DrawDecoration`, `scene::SceneDecorationStyle`. Draws a text decoration line inside a band. |
| `terminal::render::DrawGlyphRun` | `DrawGlyphRun::new(run: GlyphRun) -> DrawGlyphRun` | Re-export of `scene::DrawGlyphRun`. Wraps a glyph run as a scene command payload. |
| `terminal::render::DecorationKind` | `enum DecorationKind { Underline, Strikethrough, WavyUnderline }` | Re-export of `scene::DecorationKind`. Selects the shape of a text decoration. |
| `terminal::render::SpriteKind`, `terminal::render::SpriteSource` | `enum SpriteKind { Mono, Subpixel, Poly }`; `struct SpriteSource { atlas: u32, region: Rect, phase: f32 }` | Re-export of `scene::SpriteKind`, `scene::SpriteSource`. Select the atlas region of a sprite and how its texels are sampled. |
| `terminal::render::LayerScope`, `terminal::render::LayerEffect` | `struct LayerScope { clip, clip_radii, effect: LayerEffect, opacity }`; `enum LayerEffect { Identity, Transform(Transform), LayerBlur { sigma } }` | Re-export of `scene::LayerScope`, `scene::LayerEffect`. Open an offscreen layer with a clip, an opacity, and an effect applied when the layer composites. |
| `terminal::render::PaintFill`, `terminal::render::PatternKind` | `enum PaintFill { Solid(Rgba8), LinearGradient(LinearGradient), Pattern(PatternKind) }`; `enum PatternKind { Slash { color, period, line_width }, Checker { color_a, color_b, cell } }` | Re-export of `scene::PaintFill`, `scene::PatternKind`. Select how a shape is filled. |
| `terminal::render::LinearGradient`, `terminal::render::GradientStop` | `LinearGradient::new(start: Point, end: Point, stops) -> LinearGradient`; `GradientStop::new(offset: f32, color: Rgba8)` | Re-export of `scene::LinearGradient`, `scene::GradientStop`. Define a linear gradient from start to end through color stops. |
| `terminal::render::Stroke`, `terminal::render::StrokeStyle`, `terminal::render::StrokeAlign` | `struct Stroke { color, width, style: StrokeStyle, align: StrokeAlign }`; `StrokeStyle { Solid, Dashed(f32) }`; `StrokeAlign { Centered, Inside, Outside }` | Re-export of `scene::Stroke`, `scene::StrokeStyle`, `scene::StrokeAlign`. Describe the outline of a shape: color, width, dash, and position relative to the edge. |
| `terminal::render::Shadow`, `terminal::render::ShadowKind` | `struct Shadow { color: [f32; 4], offset: [f32; 2], blur, spread, kind: ShadowKind }`; `ShadowKind { Outer, Inner, Glow }` | Re-export of `scene::Shadow`, `scene::ShadowKind`. Describe a box shadow drawn outside, inside, or around a rounded rectangle. |
| `terminal::render::emit_scene` | `fn emit_scene(cx: &Cx, root: NodeId) -> CommandScene` | Re-export of `scene::emit_scene`. Walks the laid-out widget tree from `root` and returns its draw commands. |
| `terminal::render::WgpuRenderer` | `WgpuRenderer::new() -> Result<Self, RendererError>`; `render_command_scene(&mut self, surface: &mut WgpuSurface, scene: &CommandScene, clear_color: Color, viewport: (f32, f32)) -> Result<(), RendererError>` | Owns the GPU device and pipelines and draws a `CommandScene` to a surface. |
| `terminal::render::WgpuSurface` | `size(&self) -> PhysicalSize`; `format(&self) -> wgpu::TextureFormat` | Holds a configured window surface that the renderer draws to. |
| `terminal::render::WindowHandle`, `terminal::render::into_window_handle` | `fn into_window_handle<W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static>(window: W) -> WindowHandle` | Wraps a platform window so the renderer can create a surface for it. |
| `terminal::render::ScreenSubmission`, `terminal::render::OffscreenTarget` | `struct ScreenSubmission<'a> { device, queue, view, format, viewport, scale, clear }`; `struct OffscreenTarget<'a> { device, queue, view, format, viewport, scale }` | Name an embedder-owned wgpu target that the renderer draws a scene into. |
| `terminal::render::RendererError` | `enum RendererError { NoAdapter, DeviceRequest, SurfaceCreate, NoSupportedFormat, SurfaceValidation, NotInitialized, InvalidViewport, Layer, Software(String) }` | Reports why the renderer could not start or draw a frame. |

## Module `testing`

Feature `kittest`. For tests that drive the kernel by hand; app tests use `sigil_kittest::Harness`.

| Name | Signature | Purpose |
| --- | --- | --- |
| `testing::with_cx`, `testing::with_runtime` | `fn with_cx(name: &'static str, f: impl FnOnce(&mut Cx))`; `fn with_runtime(name: &'static str, f: impl FnOnce(&Runtime))` | Run `f` inside a fresh reactive runtime, with a root `Cx` for `with_cx`. |
| `testing::tree_size` | `fn tree_size(name: &'static str, root: impl IntoNode) -> usize` | Count the nodes a tree mounts. |
| `testing::tick_intervals` | `fn tick_intervals() -> usize` | Fire every due `interval` and return how many fired. A hand-driven frame loop calls it where `Host::frame` would. |

## Module `text`

| Name | Signature | Purpose |
| --- | --- | --- |
| `text::TextConfig` | `struct TextConfig { font_family: String, font_size: f32, font_weight: u16, line_height: f32 }`; `fn text_style(&self) -> TextStyle` | Holds basic font settings and converts them to a `TextStyle`. |
| `text::prefer_system_fonts` | `fn prefer_system_fonts()` | Switches the default fonts to system fonts unless the app already set them. |
| `text::Paragraph` | `struct Paragraph` (`Clone`) | Holds a shaped, line-broken text block. Implements `ParagraphMetrics`. |

## Module `widgets`

| Name | Signature | Purpose |
| --- | --- | --- |
| `widgets::ButtonGlyph` | `enum ButtonGlyph { Text(TextContent), Icon(Icon) }`; `From` text and `From<Icon>` | What `icon_button` shows. |
| `widgets::spinner_frame` | `fn spinner_frame(tick: u64) -> &'static str` | The spinner glyph for a tick count. |
| `widgets::icon_svg` | `fn icon_svg(svg: impl Into<Rc<str>>) -> IconBuilder` | An SVG icon. Its colour comes from the tone and the active theme at paint time. |
| `widgets::TreeNode` | `struct TreeNode<K>`; `new(id, label)`, `.leading(icon)`, `.status(text, Tone)`, `.detail(text)`, `.tone(RowTone)`, `.children(iter)`, `.child(node)` | One node of a `tree`. `leading` takes an `Icon` or a text glyph. |
| `widgets::TreeBuilder` | `struct TreeBuilder<K>`; `.expanded(Signal<Vec<K>>)`, `.selected(Signal<Option<K>>)`, `.activate(impl Fn(&K))`, `.indent(cols)`, `.label(name)` | Builder of a `tree`. `indent` sets the columns per nesting level. Without `expanded` the tree keeps its own expansion; `selected` follows the keyboard cursor; without `activate`, Enter toggles folders. Takes the `WidgetLayoutExt` overrides and fills its viewport by default. |
| `widgets::TreeKey` | `trait TreeKey: Clone + Eq + Hash + 'static` | The key type of tree nodes; implemented for every such type. |
| `widgets::TreeRow` | `struct TreeRow<K> { id, depth, disclosure, selected, parent, .. }`; `has_children()` | One visible row of a flattened tree. |
| `widgets::IconSource` | `struct IconSource`; `trait IntoIconSource` | What `icon` accepts: a bundled `Icon` or any text glyph. |
| `widgets::layout_ext::LayoutOverrides` | `struct LayoutOverrides`; `const fn new() -> Self` | Width, height, grow, padding, and full-style overrides that a widget builder applies to its root layout, plus the node hooks that `WidgetLayoutExt::map_node` queues. |
| `widgets::patterns::HintEntry` | `type HintEntry<'a> = (&'a str, &'a str)` | One key and label pair for the hint overlay. |
| `widgets::ChangeKind` | `enum ChangeKind { Added, Modified, Deleted }` | Line change mark in the `code_edit` gutter. |
| `widgets::DiagSeverity`, `widgets::DiagSpan` | `enum DiagSeverity { Hint, Info, Warning, Error }`; `struct DiagSpan { line: usize, start: usize, end: usize, severity: DiagSeverity }` | Diagnostic range that `code_edit` underlines, with its severity. |
| `widgets::parse_hex`, `widgets::to_hex` | `fn parse_hex(input: &str) -> Option<Rgba8>`; `fn to_hex(color: Rgba8) -> String` | Parse `#rgb` or `#rrggbb` to an opaque color, and format a color as `#rrggbb`. |
| `widgets::PaneBuilder` | `struct PaneBuilder`; `const fn initial(self, px: f32) -> Self`; `const fn min(self, px: f32) -> Self`; `max(px)`, `grow(factor)`, `pane(node) -> PaneBuilder`, `commit() -> SplitterBuilder` | Configures one pane of a `splitter` before it is committed. `pane` commits this pane and starts the next. |
| `widgets::ColumnLayout` | `struct ColumnLayout { order: Vec<usize>, hidden: HashSet<&'static str>, widths: HashMap<&'static str, f32> }` | Column order, hidden columns, and column widths of a `TableState`. |
| `widgets::MIN_COL_WIDTH` | `const MIN_COL_WIDTH: f32 = 48.0` | Smallest width a table column can be resized to. |
| `widgets::MAX_VISIBLE`, `widgets::TOAST_TTL` | `const MAX_VISIBLE: usize = 4`; `const TOAST_TTL: Duration = Duration::from_secs(4)` | Maximum toasts shown at once and how long a toast stays before it expires. |
| `widgets::IntoList`, `widgets::virtual_list_bind` | `trait IntoList<T, R: RowRender<T>> { fn into_list(self, render: R) -> VirtualListBuilder<T, R>; }`; `fn virtual_list_bind<S, T, R: Fn(usize, &T) -> N, N: IntoNode>(items: Signal<S>, render: R) -> VirtualListBuilder<T, R>` | `IntoList` lets `virtual_list` take a `Vec<T>` or a signal of a slice. `virtual_list_bind` builds a list bound to a signal. |
| `widgets::interaction` | `mod interaction` | Hover, press, and focus state of a widget. |
| `widgets::ButtonBuilder` | `primary()`, `outline()`, `ghost()`, `soft()`, `neutral()`, `destructive()`, `icon_svg(svg)`, `activate(f)`, `size(ControlSize)`, `disabled(impl Reactive<bool>)`; `ButtonBuilder::new(label)`, `solid()`, `tone(Tone)`, `theme(Theme)`, `accessible(name)`, `pending(Signal<bool>)` | Builder of a button. Tone and fill are independent, so call order does not matter. `primary()`, `neutral()`, and `destructive()` set only the tone (`Primary`, `Neutral`, `Error`), as does `tone(Tone)`. `solid()`, `soft()`, `ghost()`, and `outline()` set only the fill: `solid()` is a filled background, `soft()` a framed tinted fill, `ghost()` no fill and no frame, `outline()` a framed ghost. With no fill set, the `Primary` and `Error` tones draw `solid()` and every other tone draws `soft()`. With no tone set, the tone is `Neutral`. `icon_svg` puts an SVG icon before the label (feature `scene`). A button below the default size, or with a height of 32 px or less, uses the `text-xs` label. `pending` disables the button and shows a spinner before the label while the signal is true. `accessible` sets the name that screen readers read. Width and height come from `WidgetLayoutExt`. |
| `widgets::CheckboxBuilder` | `struct CheckboxBuilder`; `indeterminate(Signal<bool>)`; `CheckboxBuilder::new(Signal<bool>)`, `label(text)`, `change(Fn(bool))`, `disabled(impl Reactive<bool>)`, `tone(Tone)`, `theme(Theme)` | Builder of a checkbox. A 16 px box with an `on-accent` mark. `indeterminate` draws a dash while the signal is true. Table selection uses the same box. |
| `widgets::ToggleBuilder` | `struct ToggleBuilder`; `label(text)`, `accessible(name)`; `ToggleBuilder::new(Signal<bool>)`, `change(Fn(bool))`, `disabled(impl Reactive<bool>)`, `theme(Theme)` | Builder of a switch. `accessible` names the switch for assistive tech without visible text. The off track is `gray-6` with a `gray-11` thumb; a disabled off track is `gray-4`. |
| `widgets::SegmentedBuilder` | `struct SegmentedBuilder`; `label(name)` | Builder of a segmented control. The default height is 32 px; set it with `WidgetLayoutExt::height`. |
| `widgets::SidebarBuilder` | `struct SidebarBuilder`; `resizable(min: f32, max: f32)`, plus `Paint` and `WidgetLayoutExt` | Builder of a sidebar. `Paint` methods paint the whole column. `resizable` adds a drag handle on the inner edge; Left and Right resize it when the handle has focus, and Home and End jump to the limits. |
| `widgets::RadioGroupBuilder` | `struct RadioGroupBuilder`; `RadioGroupBuilder::new(Signal<String>)`, `option(key, label)`, `disabled(impl Reactive<bool>)`, `tone(Tone)` | Builder of a radio group. Each `option` adds one radio button. The selected signal holds the `key` of the selected option. |
| `widgets::SliderBuilder` | `struct SliderBuilder`; `step(f32)`, `length(f32)`, `label(text)`, `marks(labels)`, `change(fn)`; `SliderBuilder::new(Signal<f32>, RangeInclusive<f32>)`, `orientation(Orientation)`, `disabled(impl Reactive<bool>)`, `glass()`, `theme(Theme)` | Builder of a slider. `marks` draws one label per stop under a horizontal track, spaced evenly from the minimum to the maximum; the current stop is highlighted and a click on a label moves to it. `glass()` draws a translucent gradient track. |
| `widgets::TextFieldBuilder` | `struct TextFieldBuilder`; `autofocus()`, `focus_when(Signal<u64>)`, `focus_caret(FocusCaret)`; `TextFieldBuilder::new(Signal<String>)`, `placeholder(text)`, `change(Fn(&str))`, `submit(Fn(&str))`, `blur(Fn())`, `disabled(impl Reactive<bool>)`, `invalid_when(impl Reactive<bool>)`, `password()`, `reveal(Signal<bool>)`, `clipboard(impl Clipboard)`, `caret_visible(&mut self) -> Signal<bool>`, `scroll_offset(&mut self) -> Signal<f32>` | Builder of a text input. `autofocus()` focuses the field when it mounts. `focus_when` focuses it each time the signal changes after mount. `submit` runs with the value on Enter. `blur` runs when the field loses focus. `password()` masks the text; `reveal` shows it while the signal is true. `caret_visible` and `scroll_offset` return the signals of the caret blink and the horizontal scroll. |
| `widgets::TextEditBuilder` | `struct TextEditBuilder`; `TextEditBuilder::new(Signal<String>)`, `placeholder(text)`, `change(Fn(&str))`, `submit(Fn(&str))`, `submit_on(SubmitKey)`, `disabled(impl Reactive<bool>)`, `state(TextEditState)`, `caret_position(Signal<usize>)`, `caret_visible(&mut self) -> Signal<bool>`, `chrome(bool)`, `content_padding(horizontal, vertical)`, `clipboard(impl Clipboard)` | Builder of a text editor. Its default height is 160 px in a window and 5 rows in a terminal; set another with `.height(..)`, such as `.height(rows(3.0))`. Enter inserts a new line and `submit` runs with the value on Ctrl+Enter; `submit_on(SubmitKey::Enter)` swaps them, so Enter submits and Shift+Enter inserts a new line, as a chat composer does. The placeholder shows whenever the value is empty, focused or not. `state` takes the value, caret, and undo state from a `TextEditState`. `chrome(false)` draws the editor without its frame and background. |
| `widgets::TextEditState` | `TextEditState::new(value, caret: Signal<usize>)` | Caret and undo state of a text editor. |
| `widgets::FocusCaret` | `enum FocusCaret { Keep, End, SelectAll }` | Where `TextFieldBuilder::focus_caret` puts the caret when `autofocus` or `focus_when` focuses the field. |
| `widgets::NumberFieldBuilder` | `struct NumberFieldBuilder<T>`; `disabled(impl Reactive<bool>)`, `invalid_when(impl Reactive<bool>)`, `text_field() -> TextFieldBuilder` | Builder of a numeric input. `text_field()` returns the inner text field, invalid while the text is out of range, for further text field options. |
| `widgets::DateFieldBuilder` | `struct DateFieldBuilder`; `disabled(impl Reactive<bool>)`, `invalid_when(impl Reactive<bool>)`, `text_field() -> TextFieldBuilder` | Builder of a date input. `text_field()` returns the inner text field, invalid while the text is not a date. |
| `widgets::ColorFieldBuilder` | `struct ColorFieldBuilder`; `disabled(impl Reactive<bool>)`, `invalid_when(impl Reactive<bool>)` | Builder of a colour input. |
| `widgets::ChipBuilder` | `struct ChipBuilder`; `tone(impl Reactive<Tone>)`, `activate(f)`; `into_node_builder() -> NodeBuilder`, `layout(LayoutStyle) -> NodeBuilder` | Builder of a chip. A signal or closure passed to `tone` retints the chip when it changes. `layout` returns a `NodeBuilder`, so it comes last. |
| `widgets::InteractiveBuilder` | `struct InteractiveBuilder`; `focusable(bool)`, `disabled(impl Reactive<bool>)`, `into_node_builder() -> NodeBuilder` | Builder of an interactive node. |
| `widgets::interaction::InteractionState` | `InteractionState::new(enabled: bool)` | Hover, press, and focus signals. |
| `widgets::DropdownBuilder` | `struct DropdownBuilder`; `DropdownBuilder::new(Signal<usize>, Vec<String>)`, `change(Fn(usize))`, `label(text)`, `disabled(impl Reactive<bool>)` | Builder of a select box. |
| `widgets::MenuBuilder` | `struct MenuBuilder`; `host(owner)`; `MenuBuilder::new(Signal<bool>)`, `item(MenuItem)`, `separator()`, `label(text)`, `anchor(x, y)` | Builder of a popup menu. Item actions run under the owner current when the menu was built, or under `host`. `label` names the menu for screen readers; the default is "Menu". `anchor` sets the position of the menu. |
| `widgets::MenuBarBuilder` | `struct MenuBarBuilder`; `menu(label, IntoIterator<Item = MenuItem>)` | Builder of a menu bar. |
| `widgets::CommandMenuBuilder` | `struct CommandMenuBuilder`; `placeholder(text)`, `item(label, Fn())`, `disabled(label)`, `submenu(label, FnOnce(SubmenuBuilder) -> SubmenuBuilder)`, `submit(Fn(&str))`, `cancel(Fn())` | Builder of a command palette. `disabled(label)` adds an item that cannot run. `submit` runs with the label of each item that runs. `cancel` runs when the user leaves the top level. |
| `widgets::SubmenuBuilder` | `struct SubmenuBuilder`; `item(label, Fn())`, `disabled(label)`, `submenu(label, FnOnce(Self) -> Self)` | Builder of a command palette submenu. |
| `widgets::FuzzyMatch` | `struct FuzzyMatch { score: usize, positions: Vec<usize> }` | Result of `fuzzy_score`. |
| `widgets::TableBuilder` | `struct TableBuilder<Row>`; `framed(bool)`, `row_activate(impl Fn(&Row) + 'static)`, `selectable_checkboxes()`; `density(Density)`, `striped()`, `row_height(len)`, `header_height(px)`, `viewport_height(len)`, `viewport_width(px)`, `viewport_fill()`, `scroll_bind(Signal<f32>)`, `row_menu(Fn(&Row) -> Vec<MenuItem>)`, `empty(Fn() -> NodeBuilder)`, `disabled(impl Reactive<bool>)` | Builder of a table. `selectable_checkboxes()` adds a leading selection box to each row. `row_activate` runs on a plain left click on a row, including its padding but not its selection box, and on Enter while the row has focus. It leaves the selection as the click set it. `framed(false)` drops the rounded outer border so the table runs edge to edge. The header fills with `table-header`; row dividers use `border-subtle`. Selected rows fill with `accent-5`, and `accent-6` under hover or press. `row_menu` opens a context menu for a row. `empty` builds the content shown when no row is visible. `viewport_fill()` takes the height of the parent. |
| `widgets::ColAlign` | `enum ColAlign { Start, End }` | Column alignment. |
| `widgets::VirtualListBuilder` | `struct VirtualListBuilder<T, R>`; `measured()`, `estimate(len)`, `scroll_bind(Signal<f32>)`, `follow_row(Signal<usize>)`, `content_height_bind(Signal<f32>)`, `max_scroll_bind(Signal<f32>)`, `cursor(Signal<usize>)`, `identified()`, `follow_end(bool)`, `initial_scroll(ScrollAnchor)`, `scroll_into_view(index)`; `row_height(len)`, `item_height(Fn(usize, &T) -> f32)`, `overscan(pages)`, `key_by(Fn(usize, &T) -> u64)`, `viewport_height(len)`, `viewport_width(px)`, `viewport_fill()`, `shrink_to_content()`, `scrollbar(bool)`, `scroll_x_bind(scroll_x, max, width)`, `keyboard(bool)`, `selection(SelectionMode)`, `selection_state(&mut self) -> Signal<SelectionState>`, `change(Fn(usize))` | Builder of a virtual list. `cursor` shares the keyboard-focused row with a signal and keeps that row in view; arrow keys write it. `identified()` keys rows by `Identifiable::id`, so selection and row state follow a row when the list changes. `follow_end(true)` opens the list at the last row and keeps it pinned there while rows are added, until the user scrolls up. `follow_row` scrolls the minimum distance to keep the row in a read-only or derived signal visible, and wins over `cursor` for scrolling; while that row is the last row, it also pins the end like `follow_end`. `initial_scroll` and `scroll_into_view` set the start position only. `measured()` lays rows out at the list width and caches their heights. The two extent binds receive the content height and the largest scroll offset. `item_height` gives each row its own height. `key_by` keys rows by the value it returns. `change` runs with the first selected index each time the selection changes. |
| `widgets::RowRender` | `trait RowRender<T> { type Row: IntoNode; fn row(&self, index: usize, item: &T) -> Self::Row; }` | Row renderer of a virtual list. Every `Fn(usize, &T) -> N` with `N: IntoNode` implements it. |
| `widgets::ScrollAnchor` | `enum ScrollAnchor { Top, Bottom, Index(usize) }` | Where a virtual list stays scrolled. |
| `widgets::SelectionState` | `SelectionState::new(mode: SelectionMode)`; `click(index)`, `ctrl_click(index)`, `shift_click(index)`; `contains(index) -> bool`, `selected_indices() -> impl Iterator<Item = usize>`, `mode() -> SelectionMode` | Selected indexes of a list. |
| `widgets::ListColumn` | `ListColumn::new(text, width)`; `start()`; `From<(TextContent, f32)>` | One fixed-width column of a list row. Columns align to the end unless `start()` is set. |
| `widgets::RowTone` | `enum RowTone { Normal, Muted, Header }` | Text tone of a list row. `Muted` dims an unavailable row. `Header` labels columns and takes no hover or selection. |
| `widgets::TabsBuilder` | `struct TabsBuilder`; `tab(label, component)`, `labels(impl Reactive<Vec<S>>)`, `active(Signal<usize>)`, `pill()`, `justify_list(JustifyContent)`, `into_container() -> NodeBuilder`; `line()`, `theme(Theme)`, `disabled(&[usize])`, `closeable(bool)`, `close(Fn(usize))`, `reorderable(bool)`, `order(Signal<Vec<usize>>)` | Builder of tabs. `labels` adds tabs without panels, for a strip that drives content elsewhere. `active` binds the selected index; without it the tabs keep their own. `pill()` draws pill tabs. `justify_list` aligns the tab row. `closeable(true)` shows a close button on each tab; it calls `close` with the tab index, and the app removes the tab. `reorderable(true)` lets the user drag tabs; `order` holds the tab order. `disabled` takes the indexes of the disabled tabs. |
| `widgets::FieldBuilder` | `struct FieldBuilder<C>`; `error(impl Reactive<Option<String>>)` | Builder that `field` returns. |
| `widgets::FieldControl` | `trait FieldControl { fn invalid_when(self, impl Reactive<bool>) -> Self; }` | A control that can paint an invalid state: `text_field`, `number_field`, `color_field`. `field(..).error(..)` needs one. |
| `widgets::CodeEditBuilder` | `struct CodeEditBuilder`; `highlights(Signal<Vec<Vec<HlSpan>>>)`, `diagnostics(Signal<Vec<DiagSpan>>)`, `gutter(Signal<Vec<Option<ChangeKind>>>)`, `selection(Signal<Option<(usize, usize)>>)`, `key(Fn(&mut KeyEvent))`, `row_height(f32)`, `disabled(impl Reactive<bool>)` | Builder of a code editor. `selection` holds the selection anchor as (line, column). `key` runs for key events in the editor and makes Tab stay in it. |
| `widgets::HlSpan` | `struct HlSpan { start: usize, end: usize, token: &'static str }` | A highlight span in a code line. |
| `widgets::Highlight` | `struct Highlight { start: usize, end: usize, token: Token }` | One syntax mark: a byte range and its theme token. |
| `widgets::MarkdownEditBuilder` | `struct MarkdownEditBuilder`; `disabled(impl Reactive<bool>)` | Builder of a markdown editor. |
| `widgets::DiffBuilder` | `struct DiffBuilder`; `change(f: impl Fn(DiffView))`; `view(DiffView)`, `path(text)`, `status(text)`, `max_rows(n)` | Builder of a diff view. `change` shows one button per `DiffView` and calls `f` with the one the user picks. `path` shows in the header; `status` shows as a line under the rows. |
| `widgets::PanelBuilder` | `struct PanelBuilder`; `PanelBuilder::new()`, `title(text)`, `child(impl IntoNode)`, `padding(px)`, `elevation(tier)` | Builder of a panel. `elevation` takes 0 to 3. |
| `widgets::SplitterBuilder` | `struct SplitterBuilder`; `child(node, initial)`, `pane(node)`, `child_constrained(node, initial, min, max)`; `min_size(index, px)`, `max_size(index, px)`, `snap_points(handle, &[f32])`, `snap_threshold(px)`, `sizes_bind(Signal<Vec<f32>>)` | Builder of a splitter. `child_constrained` is `pane(node).initial(initial).min(min).max(max).commit()`. `sizes_bind` writes the pane sizes to the signal. A dragged handle snaps to a snap point within `snap_threshold`; the default is 8. |
| `widgets::ScrollBuilder` | `struct ScrollBuilder` (converts into `NodeBuilder`); `extent(f32 or Signal<f32>)`, `fill()`, `fit()`, `follow_end(bool)`; `scroll_bind(Signal<f32>)` | Builder of a scroll area. `fill()` takes the parent's height. `fit()` takes the content height up to the parent's height, then scrolls. `follow_end(true)` opens the area scrolled to the bottom and keeps it pinned there while the content grows, until the user scrolls up. |
| `widgets::HintOverlayBuilder` | `struct HintOverlayBuilder`; `HintOverlayBuilder::new(&[HintEntry])`, `entry(chord, description)` | Builder of a key hint overlay. |
| `widgets::TooltipBuilder` | `struct TooltipBuilder`; `TooltipBuilder::new(label)`, `child(target)`, `shortcut(Option<text>)`, `delay(ms)`, `clamp_top()`, `theme(Theme)` | Builder of a tooltip. `child` is the node that shows the tooltip on hover. `clamp_top()` shows the tooltip below the target instead of above. |
| `widgets::ToastEntry` | `struct ToastEntry { level: ToastLevel, message: String }` | One queued toast. |
| `widgets::ToastLevel` | `enum ToastLevel { Info, Success, Warning, Error }` | Toast severity. |
| `widgets::ProgressBuilder` | `struct ProgressBuilder`; `tone(t)`, `label(name)`, `indeterminate()`; `ProgressBuilder::new(Signal<f32>, max)` | Builder of a progress bar. `tone` colours the fill. `WidgetLayoutExt::height` sets the track height (default 8 px). In a terminal, a height under one cell draws the bar as a `─` line. |
| `widgets::GaugeBuilder` | `struct GaugeBuilder`; `GaugeBuilder::new(Signal<f32>)`, `max(f32)`, `label(text)`, `warn(ratio)` | Builder of a gauge. `warn` paints the fill in the warning colour while value / max is below the ratio. |
| `widgets::ChartBuilder` | `bar()`, `line()`, `labels(impl IntoIterator<Item = impl Into<String>>)`, `series(name, values: impl Reactive<Vec<f32>>, tone)`; `tone(Tone)` | Builder of a chart. `WidgetLayoutExt::width` and `height` set its size (default 280×180, minimum 80×48). |
| `style::ControlSize` | `enum ControlSize { Compact, Default, Wide, Grow }` | Size step of a control. `Metrics` and `Theme` map it to height, width, padding, gap, and radius. |
| `widgets::ChartMark` | `enum ChartMark { Bar, Line, Candle }` | Chart mark. |
| `widgets::SparklineBuilder` | `range(low, high)`, `tone(t)`, `tones(impl IntoIterator<Item = Option<Tone>>)`, `mark(SparkMark)`, `fill(SparkFill)`, `rows(n: usize)` | Builder of a sparkline. Without `range`, the scale runs from the lower of 0 and the minimum to the maximum. `rows(n)` draws a filled area graph `n` text rows tall, bottom-aligned. `tones` sets an optional tone per value that overrides `tone` for its column; a braille cell takes the tone of its right value, else its left. |
| `widgets::SparkMark` | `enum SparkMark { Block, Braille }` | Sparkline glyphs. `Block` (default): one column per value, 8 levels per row. `Braille`: two values per cell, 4 levels per row. |
| `widgets::SparkFill` | `enum SparkFill { Solid, Shade }` | Body of a multi-row sparkline. `Solid` (default): every cell in the tone. `Shade`: the body in a dim step of the tone and the top cell of each column solid, so a steady high value reads as a line. |
| `widgets::DotBuilder` | `struct DotBuilder`; `DotBuilder::new(Tone)`, `size(ControlSize)` | Builder of a status dot. |
| `widgets::IconBuilder` | `tone(t)`, `size(s)`, `px(px)`, `strong()`; `IconBuilder::new(glyph)`, `label(text)` | Builder of an icon. `px` sets an exact icon size. `strong` uses the high-contrast step of the tone scale. |
| `widgets::PathCmd` | `enum PathCmd { MoveTo, LineTo, QuadTo, CubicTo, Close }` | One path command. |
| `widgets::PathFill` | `enum PathFill { Solid, Theme, Linear, Radial }` | Path fill paint. `Theme(Token)` resolves at paint time. |
| `widgets::PathColor` | `enum PathColor { Rgba(Rgba8), Token(Token) }` | Colour of `fill` and `stroke`. A `Token` follows the active theme. |
| `widgets::DRAG_THRESHOLD_PX` | `const DRAG_THRESHOLD_PX: f32 = 4.0` | Pointer travel before a drag starts. |
| `widgets::DROP_TARGET_CLASS` | `const DROP_TARGET_CLASS: &str` | Class on a drop target under the pointer. |


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
| Embed the tree in another engine | `Runtime::scope`, `Host::new`, `Host::mount` |
| Patch the running app when the code changes | feature `hot`, `dx serve --hot-patch` ([Hot reload](guide/hot-reload.md)) |
| Route an input event to an embedded tree | `Host::route(&mut EventRouter, ShellEvent)` |
| Run an accessibility action on an embedded tree | `Host::perform(&mut EventRouter, NodeId, &accesskit::ActionRequest)` |
| Lay out and paint an embedded tree | `Host::layout`, then `Host::emit(viewport)` for the `CommandScene`; `Host::scene(viewport)` does both |
| Draw a scene onto a cell grid | `sigil::terminal::raster(&scene, host.theme(), cols, rows, cell)` (feature `terminal`) |
| Get the rectangles of host-drawn surfaces | `surface(key)` in the tree, then `Host::surfaces` |
| Get the accessibility tree of an embedded tree | `Host::accessibility_tree` |
| Pump external events into the terminal loop, or run code after each frame | `sigil::run(config.terminal().terminal_hooks(Hooks { .. }), tree)` |
| `$EDITOR` | `sigil::terminal::suspend(f) -> io::Result<R>` |
| Commit lines above the inline live region into scrollback | `sigil::terminal::print_above(text) -> io::Result<()>` |
| `terminal::print_above`, `NotLive`, `FullScreen`, `io::ErrorKind::Unsupported`, `err.get_ref()?.downcast_ref()` | `sigil::terminal::PrintAboveError` |
| Set the terminal window title | `sigil::terminal::set_title(title) -> io::Result<()>` |
| Ring the terminal bell, with an optional desktop notification | `sigil::terminal::bell(note: Option<&str>) -> io::Result<()>` |
| Send a terminal desktop notification | `sigil::terminal::notify(title, body) -> io::Result<()>` |
| Show progress in the terminal tab | `sigil::terminal::progress(Some(percent)) -> io::Result<()>` |
| Handle pasted text on a node | `.on_paste(f)` from `NodeEventsExt` |
| Check whether the terminal window has focus | `sigil::terminal::focused() -> bool` |
| Send Ctrl+C to the tree instead of exiting | `AppConfig::quit_on_ctrl_c(false)` |
| Set the terminal frame interval (default 33 ms) | `AppConfig::frame_interval(Duration)` |
| `Drags`, `Off` | `sigil::terminal::set_mouse_capture(yes) -> io::Result<()>` |
| Pick which terminal mouse reports the app receives | `sigil::terminal::set_mouse_tracking(tracking: MouseTracking) -> io::Result<()>` |
| Render one terminal frame to a string | `sigil::terminal::present(tree, cols, rows, events)` |
| Drive the terminal host one step at a time | `sigil::terminal::step`, `Host::cells` |
| Detect the terminal graphics protocol | `sigil::terminal::detect`, `sigil::terminal::Protocol` |
| Bind a key on a node | `.key(shortcut, handler)` from `NodeEventsExt` |
| Bind a key that fires from anywhere | `.shortcut(shortcut, handler)` from `NodeEventsExt` |
| See every key before the focused descendant | `.key_capture_any(handler)`; call `event.stop_propagation()` to keep the key from the target |
| Run a callback on a fixed period | `interval(Duration, f) -> IntervalId`, `sigil::reactive::cancel_interval` |
| Load data on demand and show loading and errors | `task(make)`, `Task::run`, `Task::cancel`, `Task::status`, `kernel::match_status` |
| Stream results from a backend thread | `signal_writer(signal)`, `SignalWriter::update` |
| Apply a theme | `parse_theme`, `set_theme`, `watch_theme`, `AppConfig::theme_file` |
| Name a theme colour in code | `sigil::style::Token` (`Token::Surface.bg()`, `Token::Ring.color()`) |
| Theme a table header or row divider | `Token::TableHeader` (`table-header`), `Token::BorderSubtle` (`border-subtle`) |

## Terminal rasteriser

The terminal host draws the same `CommandScene` as the window. Widgets
do not branch on the host. The rasteriser maps each command to cells:

| Scene command | Cells |
| --- | --- |
| `Rect` | Cell background. A leaf rect fills with a full block. A rect thinner than one cell draws `─` or `│`. |
| `Rect`, `scene::CellBorder` | Box-drawing glyphs on each side whose width is above zero. A rounded border uses rounded corners. |
| `scene::GlyphRun` | Text cells with bold and italic. `TextFlow` selects one line, wrap, or ellipsis. |
| `Image` | The detected graphics protocol, or sextant glyphs. The crop and the clip apply. |
| `PushClip`, `PushLayer` | Clip the commands inside. |
| `SaveLayer`, `PushLayer` | Blend fills, borders, and text against the cell below, with the active theme. |
| Shadow, gradient, pattern, sprite, decoration | Not drawn. |

TopLayer content comes last in the scene, so it draws above clipped
content.

## Errors

| Type | Returned by | Meaning |
| --- | --- | --- |
| `sigil::Error` | `sigil::run` | Opaque application error. `Error::kind` returns an `ErrorKind` (`Window`, `Gpu`, `Font`, `Io`, `Task`, `Config`, `Other`). `Error::new` wraps any `std::error::Error` as `Other`; `Error::with_kind` sets the kind. `Error::source_error` returns the inner error. |
| `tasks::ScheduleError` | `SignalWriter::set`, `SignalWriter::update` | `Closed`: the dispatcher is closed. `Full`: the dispatcher queue is full. |
| `layout::LayoutError` | `Host::layout`, `Host::scene`, `sigil::terminal::present`, `sigil::terminal::step` | The layout pass failed. It converts into `sigil::Error`. |
| `sigil_sign::SignError` | `sign_file`, `verify_file` | `KeyFile`, `Key`, `Package`, `Envelope`, `TrustedComment`, or `PasswordRequired`: the key, the package, the signature, or the signed file name or version failed, or an encrypted key had no password. |
| `sigil_sign::PackError` | `macos_app`, `windows_dir` | `Missing`, `Io`, `Codesign`, `Escape`, `Symlink`, `Occupied`, or `NotFile`. |

## Test harness

Package `sigil-kittest`, library `sigil_kittest`. Guide: [Testing](guide/testing.md).

| Name | Signature | Purpose |
| --- | --- | --- |
| `sigil_kittest::Harness` | `Harness::new(app: impl IntoNode + 'static) -> Harness`; `.size(cols: u16, rows: u16)`; `.pixels(width: u32, height: u32)` | Mounts a tree on a headless host with a task runtime installed. `size` uses the terminal cell host (default 80×24); `pixels` uses the window host and renders offscreen through wgpu. Each frame applies finished task results. |
| `Harness::gpu` | `fn gpu(self, width: u32, height: u32, scale: f32) -> Self` | Paint every frame through the GPU renderer into an offscreen texture at `scale` and wait for the GPU, so profiles include emit, submit, and GPU time. Without a GPU adapter it falls back to `pixels`. Every harness frame is a profile frame. |
| `Harness::send`, `Harness::settle`, `Harness::advance` | `fn send(&mut self, events: impl IntoIterator<Item = ShellEvent>) -> &mut Self`; `fn settle(&mut self) -> &mut Self`; `fn advance(&mut self, by: Duration) -> &mut Self` | Route events through `Host::frame` and run frames until no signal write or animation is pending (at most `SETTLE_SAFETY_CAP` frames). `advance` moves the virtual interval clock and runs 1/60 s frames across `by`. |
| `Harness::click`, `hover`, `scroll`, `drag` | `fn click(&mut self, target: &(impl Target + ?Sized)) -> &mut Self`; `fn scroll(&mut self, target: &(impl Target + ?Sized), lines: f32)`; `fn drag(&mut self, from: &(impl Target + ?Sized), to: &(impl Target + ?Sized))` | Pointer actions at the centre of a labelled control, a `Node`, or a `Point` (layout points). A label waits up to `EXPECT_TIMEOUT` for its control. Each action settles. |
| `Harness::press`, `press_with`, `type_text`, `type_into`, `focus_next`, `focus_previous` | `fn press(&mut self, key: impl Into<LogicalKey>) -> &mut Self`; `fn press_with(&mut self, key, modifiers: ModifiersState)`; `fn type_text(&mut self, text: &str)`; `fn type_into(&mut self, label: &str, text: &str)` | Key down and up, one key per character, or Tab and Shift+Tab. `type_into` clicks the control with that label, then types. Each action settles. |
| `Harness::expect_text` | `fn expect_text(&mut self, text: &str) -> &mut Self` | Wait until a node's name or value is `text`, running frames that apply finished tasks, up to `EXPECT_TIMEOUT`; then fail with the tree dump. |
| `Harness::get`, `get_by_role`, `get_by_text`, `query`, `find_all`, `focused`, `refresh` | `fn get_by_role(&mut self, role: Role, name: &str) -> Node`; `fn get_by_text(&mut self, text: &str) -> Node`; `fn find_all(&mut self, query: &Query) -> Vec<Node>`; `fn refresh(&mut self, node: &Node) -> Option<Node>` | Query the accessibility tree. `get*` fails the test with the tree dump unless exactly one node matches. Text runs and hidden nodes are skipped. |
| `Harness::text`, `accessibility_snapshot`, `accessibility_tree`, `image`, `host` | `fn text(&mut self) -> String`; `fn image(&mut self) -> Rendered`; `fn host(&mut self) -> (&mut Host, &mut EventRouter)` | The terminal grid text (the accessibility dump on a pixel harness), the tree, the offscreen image, or the host for anything else. |
| `Harness::snapshot`, `snapshot_with`, `sigil_kittest::SNAPSHOT_MAX_DELTA_PCT` | `fn snapshot(&mut self, name: &str)`; `fn snapshot_with(&mut self, name: &str, max_delta_pct: f32)`; `const SNAPSHOT_MAX_DELTA_PCT: f32 = 0.5` | Compare with `tests/goldens/<name>.txt` (cells) or `<name>.png` (pixels). `SIGIL_REGEN_GOLDENS=1` rewrites them. |
| `sigil_kittest::Node` | `role()`, `name()`, `value()`, `numeric()`, `bounds()`, `center()`, `is_focused()`, `is_checked()`, `is_selected()`, `is_expanded()`, `is_disabled()` | A snapshot of one accessibility node at query time. The name follows `labelled_by` when the node has no label. |
| `sigil_kittest::Query` | `Query::role(role)`, `Query::text(text)`, `.name(name)` | Match by role, accessible name, or text (name or value). |
| `sigil_kittest::Target` | `trait Target { fn point(&self, ui: &mut Harness) -> Point }` for `str`, `Node`, and `Point` | Where a pointer action lands. A `str` is a label: the one control whose name or value it is, else the one node. A label on two controls fails. |
| `sigil_kittest::EXPECT_TIMEOUT` | `const EXPECT_TIMEOUT: Duration` (5 s) | How long `expect_text` and a label target wait. |
| `sigil_kittest::assert_text_matches_golden` | `fn assert_text_matches_golden(actual: &str, golden: &Path)` | Text golden compare with the same regen switch as PNG goldens. |
| `sigil_kittest::input` | `pointer_move`, `pointer_drag`, `pointer_down`, `pointer_up`, `click`, `key_event`, `key_down`, `chord`, `press`, `type_char`, `type_text`, `tab`, `scroll_lines`, `window_blur`, `ime_event` | The `ShellEvent` builders the harness sends. Coordinates are layout points. |

## Inspection

With the `inspect` cargo feature, an app exposes a local control socket
when `SIGIL_INSPECT` is set. `SIGIL_INSPECT=1` binds `127.0.0.1:5761`;
any other value is an address to bind. The socket speaks one JSON
object per line, one reply per request.

| `op` | Request | Reply |
| --- | --- | --- |
| `a11y::tree` | – | `nodes`: id to `{role, name, description, bounds, children, focused, selected, expanded, toggled, value, numeric}` plus `root` and `focus`. Bounds are in points on the window host and cells on the terminal host. |
| `a11y::snapshot` | – | `text`: the accessibility text dump. |
| `screenshot` | – | Terminal: `text` with the painted grid. Window: `png` (base64) plus `width`/`height` in physical pixels. |
| `surfaces` | – | `surfaces`: keyed surface holes and rects. |
| `settle` | – | `ok` after servicing; `settled` is false while the frame stays dirty or animations run. |
| `terminal::click` | `col`/`row` (terminal cells) or `x`/`y` (points), `button` | Moves, presses, and releases the pointer through the normal router. |
| `move`, `down`, `up` | same coordinates | One pointer phase. |
| `widgets::scroll` | coordinates plus `dx`/`dy` in lines | A scroll event. |
| `key` | `key` (a `NamedKey` name or one character), `ctrl`/`shift`/`alt`/`meta` | Key down and up. |
| `type` | `text` | Key events per character. |
| `profile` | `last` (frame count, default 30) | `enabled`, `frames` (index, cause, `wall_ms`, per-phase ms, counters, `backend`), and `summary` (the exit report text). |
| `profile_dump` | `path` (optional) | Writes the Chrome trace JSON now and replies with `path`. Defaults to `SIGIL_PROFILE`'s value or `sigil-profile.json`. |
| `profile_on`, `profile_off` | – | Enable or stop frame collection without restarting. |
| `hud` | `on` (default true) | Draws the frame-HUD overlay: a sparkline, the active backend with any fallback cause, and last-frame phase readout top-right. Terminal host draws cells; the desktop host overlays scene commands at paint. Enabling turns profiling on. |

Requests are serviced between frames on the UI thread, so replies
reflect the tree as last laid out. The socket is loopback by
convention; the feature is off without the environment variable.

## Conventions

- Make reactive writes on the thread that owns the runtime. From
  another thread, use `signal_writer`, or `on_message` for a channel.
- `sigil::run` installs the OS clipboard on the window host. The
  terminal host installs an OSC 52 clipboard; on a local macOS terminal
  it writes with `pbcopy` and reads with `pbpaste`. Over OSC 52 the
  clipboard cannot be read, so a text input leaves Ctrl+V to the app and
  the terminal's own paste arrives as a paste event. An embed host calls
  `install(OsClipboard)` itself.
- Public event, command, error, and config types are
  `#[non_exhaustive]`. Match them with a `_` arm. Build `AppConfig`
  with its builder methods.
- The renderer API takes `wgpu` types. `sigil::wgpu` re-exports the
  `wgpu` version that `sigil-ui` uses.
- `SIGIL_BACKEND` accepts `auto`, `wgpu`, and `cpu`. See
  [Troubleshooting](guide/troubleshooting.md).
- `sigil::profile` keys are typed. `span(Phase)` times a phase and
  `count(Counter)`, `count_add(Counter, n)`, and `gauge(Counter, v)`
  record counters. `Phase::category()` returns a `Category`. `name()`
  and `Display` give the trace text. `Frame::counter(Counter)` reads a
  counter; `Frame::phases` and `Frame::counts` hold `PhaseTime` and
  `CounterValue` pairs.
- `profile::begin_frame(BackendStatus)` opens a frame for one host. A
  `BackendStatus` is a `BackendKind` (`Gpu`, `GpuSoftware`, `Cpu`,
  `Terminal`) and a `FallbackCause` (`None`, `NoAdapter`, `DeviceError`,
  `ForcedByEnv`, `UnknownEnv`). `BackendStatus::TERMINAL` is the terminal
  host's status. `Frame::backend` holds the status of the
  host that drew that frame, so a window host and a terminal host in one
  process keep separate statuses.
- `profile::interact(Cause)` marks the input that caused a frame.
  `Cause` is `Idle`, `Pointer`, `Hover`, `Key`, `Scroll`, `Ime`,
  `Resize`, `A11y`, `Expose`, or `Input`. When several inputs land in one
  frame, the highest `Cause::rank()` wins. `Frame::cause` holds it.
- Parsers for markdown, HTML, SVG, and theme files run on the UI
  thread. See `SECURITY.md` for the input surfaces.
