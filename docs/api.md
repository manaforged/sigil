# Sigil API map

This page lists every item that `sigil` exports at the crate root and
in `sigil::prelude`. Each item has a signature, a one-line purpose, and
an example. Every example on this page compiles as a doctest of the
`sigil` crate.

The package is `sigil-ui`. The library is `sigil`. The API can change
between 0.x releases.

```toml
[dependencies]
sigil-ui = "0.1"
```

Application code imports the prelude:

```rust,no_run
use sigil::prelude::*;

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("App").terminal(), |cx: &mut Cx| cx.add(text("Hello")))
}
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
| `prelude` | `mod prelude` | The application import surface. |
| `a11y` | `mod a11y` | Accessibility tree construction. |
| `animation` | `mod animation` | Easing, springs, and keyframes. |
| `animation_runtime` | `mod animation_runtime` | Animators that drive signals each frame. |
| `app` | `mod app` | Application host, configuration, and theme files. |
| `bidi` | `mod bidi` | Bidirectional text runs. |
| `clipboard` | `mod clipboard` | Clipboard trait and the installed clipboard. |
| `events` | `mod events` | Event routing, handlers, and focus. |
| `hal` | `mod hal` | Input events, keys, and geometry types. |
| `icons` | `mod icons` | Bundled icon glyphs (feature `scene`). |
| `kernel` | `mod kernel` | Node tree, builders, and roles. |
| `layout` | `mod layout` | Layout styles and the layout engine. |
| `profile` | `mod profile` | Frame timing counters. |
| `reactive` | `mod reactive` | Signals, effects, owners, and intervals. |
| `render` | `mod render` | Offscreen rendering for embed hosts (feature `render`). |
| `renderer` | `mod renderer` | The GPU and CPU renderers (feature `scene`). |
| `scene` | `mod scene` | The `CommandScene` display list. |
| `shell` | `mod shell` | The window shell (feature `desktop`). |
| `style` | `mod style` | Styles, themes, tokens, and utility classes. |
| `tasks` | `mod tasks` | Async tasks and cross-thread dispatch. |
| `terminal` | `mod terminal` | The terminal host (feature `terminal`). |
| `testing` | `mod testing` | Test support for `sigil-kittest` (feature `kittest`, hidden). |
| `text` | `mod text` | Fonts and text shaping (feature `scene`). |
| `widgets` | `mod widgets` | Widget builders. |
| `interaction` | `mod interaction` | Hover, press, and focus state of a widget. |
| `wgpu` | `extern crate wgpu` | The `wgpu` version that the renderer uses (feature `scene`). |
| `run` | `fn run<C: Into<AppConfig>, T: TreeRoot>(config: C, tree: T) -> AppResult<()>` | Run a tree on the host that `config` selects. |
| `root` | `fn root(node: impl IntoNode) -> impl TreeRoot` | Wrap a built node as a tree root. |
| `request_exit` | `fn request_exit()` | Ask the running host to exit after this frame. |
| `is_grid` | `fn is_grid() -> bool` | `true` on a cell grid (terminal) host. |
| `viewport_size` | `fn viewport_size() -> Signal<LogicalSize>` | The viewport size, in logical px or cells. |
| `set_theme` | `fn set_theme(theme: Theme)` | Replace the active theme at the next frame. |
| `parse_theme` | `fn parse_theme(input: &str) -> Option<Theme>` | Parse a theme TOML document. |
| `watch_theme` | `fn watch_theme(path: impl Into<PathBuf>) -> AppResult<ThemeWatcher>` | Reload a theme file when it changes. |
| `ThemeWatcher` | `struct ThemeWatcher` | Guard of `watch_theme`. Drop it to stop the watch. |
| `open` | `fn open(target: impl AsRef<OsStr>) -> AppResult<()>` | Open a file or URL with the OS handler (feature `desktop`). |
| `dialog` | `mod dialog` | Native file and message dialogs (feature `dialog`). |
| `menu` | `mod menu` | Native menu bar (feature `menu`). |
| `tray` | `mod tray` | System tray icon (feature `tray`). |
| `Player` | `struct Player` | Audio file player (feature `audio`). |
| `Session` | `struct Session` | Child process in a pseudo-terminal (feature `pty`). |
| `AppConfig` | `struct AppConfig` | Run configuration: title, size, host, theme file, web canvas. |
| `AppResult` | `type AppResult<T> = Result<T, Error>` | Result of the application functions. |
| `Error` | `struct Error` | Opaque application error. |
| `ErrorKind` | `enum ErrorKind` | Class of an `Error`. |
| `PanicPolicy` | `enum PanicPolicy { Isolate, Abort }` | What a panic in a handler does. |
| `Host` | `struct Host` | Embed host: build, route, lay out, and paint one tree. `set_caps(HostCaps)` tells widgets what the host can show. |
| `HostCaps` | `struct HostCaps { hyperlinks: bool }`; `HostCaps::current()`, `HostCaps::window()` | What the running host can show. The terminal host fills it from the detected terminal. |
| `TreeRoot` | `trait TreeRoot` | A value that builds the tree, such as `fn(&mut Cx) -> NodeId`. |
| `AccessibilityTree` | `type AccessibilityTree = accesskit::TreeUpdate` | Accessibility tree of an embedded tree. |
| `WindowCommand` | `enum WindowCommand` | Close, minimize, maximize, drag, or zoom the window. |
| `ResizeDirection` | `enum ResizeDirection` | Edge or corner for `WindowCommand::DragResize`. |
| `WindowOpts` | `struct WindowOpts` | Window options that `AppConfig` collects. |
| `Anchor` | `enum Anchor { BottomCenter { margin: f32 } }` | Screen anchor of an overlay window. |
| `DefaultFonts` | `struct DefaultFonts` | Font bytes for the sans, mono, icon, and display faces. |
| `set_default_fonts` | `fn set_default_fonts(fonts: DefaultFonts)` | Replace the bundled fonts. |
| `use_system_fonts` | `fn use_system_fonts()` | Use the operating system fonts. |
| `TextTuning` | `struct TextTuning` | Text rasterisation settings. |
| `text_tuning` | `fn text_tuning() -> TextTuning` | Read the text rasterisation settings. |
| `set_text_tuning` | `fn set_text_tuning(tuning: TextTuning)` | Set the text rasterisation settings. |

```rust,no_run
use sigil::prelude::*;
use std::time::Duration;

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(text("Hello"))
}

fn main() -> sigil::AppResult<()> {
    let config: sigil::AppConfig = sigil::AppConfig::new("Demo").size(640, 400).terminal();
    let _ = (sigil::request_exit, sigil::is_grid, sigil::viewport_size);
    let _: sigil::AppResult<()> = Err(sigil::Error::with_kind(sigil::ErrorKind::Io, std::fmt::Error));
    let _ = sigil::PanicPolicy::Isolate;
    let _: fn(&mut Cx) -> NodeId = app_tree;
    let _ = sigil::root(text("static"));
    let _ = config.clone().frame_interval(Duration::from_millis(33));
    if let Some(theme) = sigil::parse_theme("base = \"dark\"\n[tokens]\naccent = \"#3c59e7\"\n") {
        sigil::set_theme(theme);
    }
    let _: fn(std::path::PathBuf) -> sigil::AppResult<sigil::ThemeWatcher> = sigil::watch_theme;
    let _ = sigil::open("https://example.com");
    let mut host = sigil::Host::new();
    host.build(app_tree);
    let _: Option<sigil::AccessibilityTree> = None;
    let _ = sigil::WindowCommand::DragResize(sigil::ResizeDirection::East);
    let _ = sigil::WindowOpts::default();
    let _ = sigil::Anchor::BottomCenter { margin: 16.0 };
    sigil::set_default_fonts(sigil::DefaultFonts::default());
    sigil::use_system_fonts();
    let tuning: sigil::TextTuning = sigil::text_tuning();
    sigil::set_text_tuning(tuning);
    let _ = sigil::interaction::InteractionState::new(true);
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
    let _ = sigil::hal::KeyPhase::Down;
    let _ = sigil::kernel::container();
    let _ = sigil::clipboard::paste();
    let _ = sigil::app::set_theme;
    sigil::run(config, app_tree)
}
```

The modules `bidi`, `icons`, `profile`, `renderer`, `shell`, and
`text` have no stable entry point in 0.1. Use them only through the
functions above.

Feature-gated root items:

```rust,no_run
#[cfg(feature = "audio")]
fn play(path: &std::path::Path) -> sigil::AppResult<()> {
    sigil::Player::new().play_path(path)
}

#[cfg(feature = "pty")]
fn shell() -> sigil::AppResult<sigil::Session> {
    sigil::Session::spawn("sh")
}

#[cfg(feature = "dialog")]
fn pick() -> Option<std::path::PathBuf> {
    sigil::dialog::open_file()
}

#[cfg(feature = "menu")]
fn menu_events() -> sigil::prelude::EventChannel<muda::MenuId> {
    sigil::menu::events()
}

#[cfg(feature = "tray")]
fn tray_events() -> sigil::prelude::EventChannel<tray_icon::TrayIconEvent> {
    sigil::tray::events()
}

#[cfg(feature = "render")]
use sigil::render as _;

#[cfg(feature = "kittest")]
use sigil::testing as _;

fn main() {}
```

## Prelude

`use sigil::prelude::*;` imports every item in this section.

### Application

| Name | Signature | Purpose |
| --- | --- | --- |
| `run` | `fn run(config, tree) -> AppResult<()>` | Same as `sigil::run`. |
| `root` | `fn root(node: impl IntoNode) -> impl TreeRoot` | Same as `sigil::root`. |
| `AppConfig` | `struct AppConfig` | Same as `sigil::AppConfig`. |
| `AppResult` | `type AppResult<T>` | Same as `sigil::AppResult`. |
| `Error` | `struct Error` | Same as `sigil::Error`. |
| `Host` | `struct Host` | Same as `sigil::Host`. |
| `HostCaps` | `struct HostCaps` | Same as `sigil::HostCaps`. |
| `TreeRoot` | `trait TreeRoot` | Same as `sigil::TreeRoot`. |
| `is_grid` | `fn is_grid() -> bool` | Same as `sigil::is_grid`. |
| `viewport_size` | `fn viewport_size() -> Signal<LogicalSize>` | Same as `sigil::viewport_size`. |
| `set_theme` | `fn set_theme(theme: Theme)` | Same as `sigil::set_theme`. |
| `parse_theme` | `fn parse_theme(input: &str) -> Option<Theme>` | Same as `sigil::parse_theme`. |
| `watch_theme` | `fn watch_theme(path) -> AppResult<ThemeWatcher>` | Same as `sigil::watch_theme`. |
| `ThemeWatcher` | `struct ThemeWatcher` | Same as `sigil::ThemeWatcher`. |
| `open` | `fn open(target) -> AppResult<()>` | Same as `sigil::open` (feature `desktop`). |
| `Player` | `struct Player` | Same as `sigil::Player` (feature `audio`). |
| `Session` | `struct Session` | Same as `sigil::Session` (feature `pty`). |
| `Runtime` | `struct Runtime`; `Runtime::scope(name, f) -> R` | Run code inside a reactive runtime. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
    let size = viewport_size();
    cx.add(text(derived(move || format!("{:?} grid={}", size.get(), is_grid()))))
}

fn main() -> AppResult<()> {
    let _: Option<(Host, ThemeWatcher, Error)> = None;
    let _ = Runtime::scope("demo", || root(text("x")));
    set_theme(parse_theme("base = \"light\"\n[tokens]\n").unwrap_or_else(Theme::light));
    let _ = (open("https://example.com"), watch_theme("theme.toml").map(drop));
    run(AppConfig::new("Demo"), tree)
}

#[cfg(feature = "audio")]
fn player() -> Player { Player::new() }
#[cfg(feature = "pty")]
fn session() -> AppResult<Session> { Session::spawn("sh") }
fn _tree_root<T: TreeRoot>(_: T) {}
```

### Nodes and text

| Name | Signature | Purpose |
| --- | --- | --- |
| `Cx` | `struct Cx`; `cx.add(node) -> NodeId` | Build context that the tree function receives. |
| `NodeId` | `struct NodeId` | Handle of a node in the tree. |
| `Node` | `struct Node` | A node in the tree, as `NodeBuilder::mutate` sees it. |
| `NodeType` | `enum NodeType` | Container, text, image, or fragment. |
| `NodeBuilder` | `struct NodeBuilder` | Builder that `container` and most widgets return. |
| `NodeExtensions` | `struct NodeExtensions` | Typed data attached to a node. |
| `IntoNode` | `trait IntoNode` | A value that builds into a node. |
| `node` | `fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode>` | Box a child of any builder type. |
| `container` | `fn container() -> NodeBuilder` | A box that holds children. |
| `fragment` | `fn fragment() -> NodeBuilder` | Children without a box of their own. |
| `text` | `fn text(source: impl Into<TextContent>) -> TextBuilder` | A text node. |
| `text_live` | `fn text_live(f: impl Fn() -> String + 'static) -> TextBuilder` | Text that `f` recomputes when its signals change. |
| `TextBuilder` | `struct TextBuilder` | Builder of a text node. |
| `TextContent` | `enum TextContent` | Static, signal, or computed text. |
| `image` | `fn image(source: ImageSource) -> sigil::kernel::ImageBuilder` | An image node from an `ImageSource`. |
| `ImageBuilder` | `alt`, `fit`, `crop`, `mutate`, plus `WidgetLayoutExt` | The one image builder. `image`, `image_rgba`, and `image_bytes` return it. |
| `ImageSource` | `ImageSource::from_rgba8(w, h, rgba) -> Option<ImageSource>` | Decoded RGBA pixels. |
| `ImageCrop` | `struct ImageCrop { top: u32, height: u32 }` | Vertical crop of an image. |
| `ObjectFit` | `enum ObjectFit { Fill, Contain, Cover }` | How an image fills its box. |
| `Surface` | `struct Surface { key: u64 }` | Marker of a host-drawn surface. |
| `TopLayer` | `struct TopLayer` | Marker of a node that paints above the tree. |
| `A11yProps` | `struct A11yProps` | Accessibility properties of a node. |
| `AccessibleName` | `enum AccessibleName` | None, static, or signal name. |
| `Role` | `accesskit::Role` | Accessibility role. |
| `Action` | `accesskit::Action` | Accessibility action. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
    let pixels = ImageSource::from_rgba8(1, 1, vec![255, 0, 0, 255]);
    let picture = pixels.map(image).map(|i| i.fit(ObjectFit::Contain));
    let heading: TextBuilder = text(TextContent::from("Title")).role(Role::Heading);
    let body = container().label(AccessibleName::Static("Body".into())).action(Action::Click);
    let marked = container().mutate(|n: &mut Node| drop((&n.kind as &NodeType, &n.a11y as &A11yProps)));
    let children: Vec<Box<dyn IntoNode>> = vec![node(heading), node(text_live(|| "live".into()))];
    let _: Option<(ImageCrop, Surface, TopLayer, &NodeExtensions)> = None;
    let mut root: NodeBuilder = fragment().child(body).child(marked);
    for child in children { root = root.child(child); }
    if let Some(picture) = picture { root = root.child(picture); }
    let id: NodeId = cx.add(container().child(root));
    id
}

fn main() {
    let _ = tree;
}
```

### Layout

| Name | Signature | Purpose |
| --- | --- | --- |
| `LayoutStyle` | `LayoutStyle::flex_column()`, `flex_row()`, `grid(cols)`, `block()` | Layout description of a node. |
| `LayoutBuilderExt` | `fn layout(self, style: LayoutStyle) -> Self` | Set the layout of a node or text. |
| `WidgetLayoutExt` | `fn layout`, `width`, `height`, `grow`, `padding` | Override the layout of a widget builder or an image. |
| `LayoutReactiveExt` | `fn position_reactive`, `size_reactive`, `bounds_reactive` | Write the laid-out position or size to a signal. |
| `ReactivePosition` | `struct ReactivePosition` | Extension that `position_reactive` attaches. |
| `BoundsReactive` | `struct BoundsReactive` | Extension that `bounds_reactive` attaches. |
| `Display` | `enum Display { Block, Flex, Grid, None }` | Display mode. |
| `FlexDirection` | `enum FlexDirection { Row, Column, RowReverse, ColumnReverse }` | Main axis of a flex box. |
| `FlexWrap` | `enum FlexWrap { NoWrap, Wrap, WrapReverse }` | Wrap mode of a flex box. |
| `AlignItems` | `enum AlignItems { Start, End, Center, Baseline, Stretch }` | Cross-axis alignment of children. |
| `AlignSelf` | `type AlignSelf = AlignItems` | Cross-axis alignment of one child. |
| `JustifyContent` | `enum JustifyContent` | Main-axis distribution of children. |
| `Position` | `enum Position { Relative, Absolute }` | Positioning mode. |
| `Overflow` | `enum Overflow { Visible, Hidden, Scroll }` | Overflow mode. |
| `Dimension` | `enum Dimension` | A length: px, percent, text length, or auto. |
| `LengthPercentage` | `enum LengthPercentage` | A px or percent length. |
| `LengthPercentageAuto` | `enum LengthPercentageAuto { Px, Percent, Auto }` | A px, percent, or auto length. |
| `TrackSize` | `enum TrackSize { Px(f32), Fr(f32), Auto, MinMax(f32, f32) }` | Size of a grid track. |
| `TextLength` | `enum TextLength { Rows(f32), Cols(f32), Em(f32) }` | A length relative to the text metrics. |
| `rows` | `const fn rows(count: f32) -> TextLength` | Height of `count` text lines. |
| `cols` | `const fn cols(count: f32) -> TextLength` | Width of `count` character cells. |
| `em` | `const fn em(count: f32) -> TextLength` | `count` times the font size. |
| `IntoPx` | `trait IntoPx { fn into_px(self) -> f32; }` | A pixel count or a `TextLength`, for widget sizes such as `width` and `row_height`. |
| `LayoutEngine` | `struct LayoutEngine`; `take_republished(&mut self) -> bool` | The layout pass. `Host` owns one. `take_republished` reports whether the last pass changed a size or bounds binding, and resets the flag. `Host` runs up to four passes per frame until no binding changes. |
| `LayoutResult` | `struct LayoutResult` | The laid-out rectangle of a node. |
| `LayoutError` | `enum LayoutError` | The layout pass failed. |
| `TextMeasure` | `trait TextMeasure` | Text measurement that the layout pass calls. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
    let size = signal((0.0_f32, 0.0_f32));
    let grid = LayoutStyle::grid(vec![TrackSize::Px(120.0), TrackSize::Fr(1.0), TrackSize::Auto]);
    let column = LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)).min_width(em(20.0));
    let row = LayoutStyle::flex_row().flex_wrap(FlexWrap::Wrap).align_items(AlignItems::Center);
    let row = row.justify_content(JustifyContent::SpaceBetween).align_self(AlignSelf::Stretch);
    let abs = LayoutStyle::block().position(Position::Absolute).inset(LengthPercentageAuto::Px(0.0));
    let abs = abs.overflow(Overflow::Hidden).display(Display::Flex).flex_direction(FlexDirection::Row);
    let _ = (Dimension::from(rows(2.0)), LengthPercentage::from(cols(1.0)), TextLength::Em(1.0));
    let _: Option<(&LayoutEngine, LayoutResult, LayoutError, ReactivePosition, BoundsReactive)> = None;
    let _: Option<&dyn TextMeasure> = None;
    let button = button("OK").width(80.0).grow(1.0);
    cx.add(
        container()
            .layout(column)
            .child(container().layout(grid).size_reactive(size))
            .child(container().layout(row).child(button))
            .child(container().layout(abs)),
    )
}

fn main() {
    let _ = tree;
    fn _ext<T: WidgetLayoutExt + LayoutBuilderExt + LayoutReactiveExt>(_: T) {}
}
```

See [Units](guide/units.md) for the rules that map `rows`, `cols`, and
`em` to pixels and cells.

### State

| Name | Signature | Purpose |
| --- | --- | --- |
| `Signal` | `struct Signal<T>`; `get`, `peek`, `set`, `update`, `with`, `track` | Reactive value. |
| `signal` | `fn signal<T: 'static>(initial: T) -> Signal<T>` | Create a signal. |
| `derived` | `fn derived<T: Clone, F: FnMut() -> T>(f: F) -> Signal<T>` | A signal that `f` recomputes. |
| `derived_eq` | `fn derived_eq<T: Clone + PartialEq, F: FnMut() -> T>(f: F) -> Signal<T>` | Like `derived`, but notifies only on a change. |
| `effect` | `fn effect<F: FnMut() + 'static>(f: F)` | Run `f` now and after each change it reads. |
| `on_cleanup` | `fn on_cleanup(f: impl FnOnce() + 'static)` | Run `f` when the current owner is disposed. |
| `batch` | `fn batch<R>(f: impl FnOnce() -> R) -> R` | Group writes into one update. |
| `untrack` | `fn untrack<R>(f: impl FnOnce() -> R) -> R` | Run `f` without subscribing the current effect to its reads. |
| `defer` | `fn defer(f: impl FnOnce() + 'static)` | Run `f` after the current effect returns, or now outside an effect. An effect uses it to consume the signal that triggers it. |
| `Reactive` | `trait Reactive<T>`; `fn read(&self) -> T` | A signal or a closure used as a value source. |
| `EventChannel` | `struct EventChannel<T>`; `emit`, `subscribe` | Send events between parts of the app. |
| `interval` | `fn interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId` | Run `callback` on a fixed period. |
| `IntervalId` | `struct IntervalId` | Handle for `sigil::reactive::cancel_interval`. |

```rust,no_run
use sigil::prelude::*;
use std::time::Duration;

fn tree(cx: &mut Cx) -> NodeId {
    let count: Signal<i32> = signal(0);
    let double = derived(move || count.get() * 2);
    let even = derived_eq(move || count.get() % 2 == 0);
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
    cx.add(text(derived(move || count.get().to_string())))
}

fn main() {
    let _ = tree;
}
```

### Tasks

| Name | Signature | Purpose |
| --- | --- | --- |
| `spawn` | `fn spawn<T, E, F: Future<Output = Result<T, E>> + Send>(future: F) -> Signal<Status<T, E>>` | Run a future off the UI thread. |
| `tasks::sleep` | `async fn sleep(duration: Duration)` | Wait inside a spawned future without blocking a thread. |
| `Status` | `enum Status<T, E> { Pending, Ok(T), Err(E), Panicked(String) }` | State of a spawned task. |
| `signal_writer` | `fn signal_writer<T: Send>(signal: Signal<T>) -> SignalWriter<T>` | A `Send` handle that writes a signal. |
| `SignalWriter` | `set(value) -> Result<(), ScheduleError>`, `update(f) -> Result<(), ScheduleError>` | Write a signal from another thread. |
| `ScheduleError` | `enum ScheduleError { Closed, Full }` | A cross-thread write was not queued. |
| `post` | `fn post(f: impl FnOnce() + Send + 'static)` | Run `f` on the UI thread. |
| `dispatcher` | `fn dispatcher() -> Dispatcher` | The queue of the UI thread. |
| `dispatch_to_owner` | `fn dispatch_to_owner(dispatcher: &Dispatcher, handle: OwnerHandle, f: F)` | Run `f` on the UI thread while the owner lives. |
| `on_message` | `fn on_message<T: Send + 'static, F: Fn(T) + 'static>(rx: impl MessageReceiver<T>, f: F)` | Run `f` on the UI thread for each message, in the current owner. `rx` is a tokio `UnboundedReceiver<T>` or a bounded `Receiver<T>`. |
| `on_message_or_close` | `fn on_message_or_close<T: Send + 'static, F: Fn(T) + 'static, C: FnOnce() + 'static>(rx: impl MessageReceiver<T>, f: F, on_close: C)` | Like `on_message`. Runs `on_close` on the UI thread after the last message, when every sender is dropped. |
| `OwnerHandle` | `OwnerHandle::current() -> Option<OwnerHandle>` | `Send` handle of a reactive owner. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
    let log = signal(String::new());
    let writer: SignalWriter<String> = signal_writer(log);
    std::thread::spawn(move || {
        let _: Result<(), ScheduleError> = writer.update(|s| s.push_str("line\n"));
    });
    let fetched: Signal<Status<u32, String>> = spawn(async { Ok::<u32, String>(42) });
    post(|| println!("on the UI thread"));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    on_message(rx, move |line| log.update(|s| s.push_str(&line)));
    let (done_tx, done_rx) = tokio::sync::mpsc::channel::<String>(16);
    on_message_or_close(done_rx, move |line| log.set(line), || println!("worker closed"));
    drop(done_tx);
    drop(tx.send("hello\n".into()));
    if let Some(owner) = OwnerHandle::current() {
        dispatch_to_owner(&dispatcher(), owner, || println!("owner alive"));
    }
    cx.add(text(derived(move || match fetched.get() {
        Status::Ok(n) => format!("{n}"),
        _ => log.get(),
    })))
}

fn main() {
    let _ = tree;
}
```

### Events and input

| Name | Signature | Purpose |
| --- | --- | --- |
| `NodeEventsExt` | `key`, `key_any`, `key_capture`, `key_capture_any`, `shortcut`, `pointer_down`, `on_scroll`, `focus_in`, ... | Attach event handlers to a `NodeBuilder`. |
| `EventHandlers` | `struct EventHandlers` | The handler set of a node. |
| `EventRouter` | `EventRouter::new()` | Routes shell events to an embedded tree. |
| `request_focus` | `fn request_focus(id: NodeId)` | Move keyboard focus to a node. |
| `request_reveal` | `fn request_reveal(id: NodeId)` | Scroll each enclosing `scroll` so a node is in view after the next layout. |
| `FocusIntent` | `enum FocusIntent { Up, Down, Left, Right }` | Directional focus move. |
| `request_window_command` | `fn request_window_command(command: WindowCommand)` | Send a command to the window. |
| `WindowCommand` | `enum WindowCommand` | Same as `sigil::WindowCommand`. |
| `ResizeDirection` | `enum ResizeDirection` | Same as `sigil::ResizeDirection`. |
| `ShellEvent` | `enum ShellEvent { Pointer, Key, Ime, Scroll, FileDrop, Window }` | An input event from the host. |
| `Shortcut` | `Shortcut::new(key, mods)`, `plain(key)`, `ctrl(key)` | A key plus modifiers. |
| `Key` | `keyboard_types::NamedKey` | A named key such as `Key::Enter`. |
| `LogicalKey` | `LogicalKey::named(key)`, `LogicalKey::character(s)` | A named key or a character. |
| `PhysicalKey` | `keyboard_types::Code` | A key position on the keyboard. |
| `ModifiersState` | `keyboard_types::Modifiers` | Modifier flags such as `CONTROL`. |
| `KeyEvent` | `struct KeyEvent { key, physical, modifiers, repeat, phase, text }` | A key press or release. |
| `KeyPhase` | `enum KeyPhase { Down, Up }` | Press or release. |
| `ImeEvent` | `enum ImeEvent { Enabled, Preedit, Commit, Disabled }` | Input method event. |
| `PointerEvent` | `struct PointerEvent` | A pointer event. |
| `PointerEventInit` | `struct PointerEventInit` | Fields to build a `PointerEvent`. |
| `PointerKind` | `enum PointerKind { Mouse, Touch, Pen }` | Pointer device. |
| `PointerPhase` | `enum PointerPhase { Down, Move, Up, Enter, Leave, Cancel }` | Pointer event phase. |
| `MouseButton` | `enum MouseButton { Left, Right, Middle, Back, Forward, Other(u16) }` | One mouse button. |
| `MouseButtons` | `struct MouseButtons`; `MouseButtons::LEFT` | Set of pressed buttons. |
| `ScrollEvent` | `struct ScrollEvent` | A wheel or trackpad scroll. |
| `ScrollDelta` | `enum ScrollDelta { Pixels(Point), Lines(Point) }` | Scroll amount. |
| `ScrollPhase` | `enum ScrollPhase { Started, Moved, Ended, Cancelled }` | Scroll gesture phase. |
| `FileDropEvent` | `struct FileDropEvent` | A file drag over or onto the window. |
| `FileDropPhase` | `enum FileDropPhase { Hovered, Dropped, Cancelled }` | File drop phase. |
| `WindowEvent` | `enum WindowEvent` | Resize, focus, and close events of the window. |
| `Point` | `struct Point { x: f32, y: f32 }` | A position in logical px. |
| `Size` | `struct Size { width: Dim, height: Dim }` | A size constraint. |
| `Dim` | `enum Dim { Fixed(f32), MinContent, MaxContent }` | One axis of a size constraint. |
| `LogicalSize` | `struct LogicalSize { width: f32, height: f32 }` | A size in logical px or cells. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
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
        .on_ime(|event: &ImeEvent| drop(matches!(event, ImeEvent::Commit(_))));
    let id = cx.add(root);
    request_focus(id);
    request_window_command(WindowCommand::DragResize(ResizeDirection::South));
    id
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
| `classes` | `classes!("p-2", "bg-surface")` | Parse utility classes into style atoms. |
| `NodeStyleExt` | `class`, `style`, `theme`, `tone`, `focus_ring`, `presence`, `animate` | Style a `NodeBuilder`. |
| `Style` | `Style::new().class(input)` | A list of style atoms with state variants. |
| `StyleAtom` | `enum StyleAtom` | One style property. |
| `Px` | `struct Px(pub i16)` | A pixel length in a style atom. |
| `StyleDimension` | `enum StyleDimension { Px(Px), Full, Fraction(u16), Auto }` | A size in a style atom. |
| `Rgba8` | `struct Rgba8 { r, g, b, a: u8 }` | An 8-bit RGBA colour. |
| `ColorAtom` | `enum ColorAtom { Rgba8(Rgba8), ThemeRef(&'static str) }` | A colour or a theme token name. |
| `ColorOrToken` | `enum ColorOrToken { Rgba(Rgba8), Token(&'static str), Atom(ColorAtom) }` | Colour argument of the style traits. |
| `Theme` | `Theme::light()`, `Theme::dark()`, `Theme::console_dark()` | Theme values. |
| `ThemeValue` | `enum ThemeValue { Color(Rgba8), Px(f32), Ms(u16) }` | One theme token value. |
| `Tone` | `enum Tone { Neutral, Primary, Info, Success, Warn, Error }` | Semantic colour family. |
| `FocusRing` | `FocusRing::static_visible(bool)` | Focus ring settings. |
| `StyleState` | `struct StyleState { disabled, hovered, pressed, focused: bool }` | Interaction state for a style variant. |
| `StateStyle` | `StateStyle::new(rest, focused, hovered, pressed, disabled)` | Atoms per interaction state. |
| `StyleVariant` | `StyleVariant::new(base, states)` | Base atoms plus `StateStyle`. |
| `NumericAtomKind` | `enum NumericAtomKind` | A numeric style property that `animate` drives. |
| `StyleAccess` | `trait StyleAccess` | Access to the `Style` of a builder. |
| `Styleable` | `fn styled(self) -> NodeStyledBuilder` | Style a `NodeBuilder` with method calls. |
| `Styled` | `trait Styled` | All the style method traits together. |
| `Fill` | `fn bg(color)`, `fn text_color(color)` | Background and text colour. |
| `Flex` | `fn flex_row()`, `flex_col()`, `hidden()` | Display and flex direction. |
| `Geometry` | `fn rounded(px)`, `border_w(px)`, `border_color(color)` | Radius and border. |
| `Sizing` | `fn w(dim)`, `h(dim)`, `min_w(dim)`, `max_w(dim)` | Width and height. |
| `Spacing` | `fn padding(px)`, `px(px)`, `py(px)` | Padding. |
| `Typography` | `fn font_size(px)`, `font_weight(w)`, `line_height(px)` | Font properties. |
| `Visual` | `fn opacity(v)`, `overflow_hidden()` | Opacity, transform, and overflow. |
| `Transitions` | `fn transition(id)`, `duration(ms)` | Style transitions. |
| `StateVariants` | `fn hover(f)`, `focus(f)`, `pressed(f)` | Style per interaction state. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
    let atoms = classes!("p-2", "rounded-md");
    let styled = container().styled().bg(Rgba8 { r: 20, g: 20, b: 30, a: 255 }).padding(8).rounded(4);
    let styled = styled.text_color(ColorOrToken::Token("on-surface")).font_weight(600).opacity(0.9);
    let styled = styled.w(StyleDimension::Px(Px(200))).flex_col().hover(|s| s.bg(ColorOrToken::Token("surface-hover")));
    let style = Style::new().class("bg-surface text-sm");
    let _ = (StyleAtom::PaddingAll(Px(4)), ColorAtom::ThemeRef("accent"), ThemeValue::Ms(150));
    let _ = StyleVariant::new(atoms, StateStyle::new(&[], &[], &[], &[], &[]));
    let _ = (StyleState { disabled: false, hovered: true, pressed: false, focused: false }, NumericAtomKind::Opacity);
    let theme = Theme::dark();
    let node = container().class(atoms).style(style).tone(Tone::Primary).focus_ring(FocusRing::static_visible(true));
    cx.add(fragment().child(node.theme(theme)).child(styled))
}

fn main() {
    let _ = tree;
    fn _traits<T: Styled + Fill + Flex + Geometry + Sizing + Spacing + Typography + Visual + Transitions + StateVariants + StyleAccess>(_: T) {}
    fn _styleable<T: Styleable + NodeStyleExt>(_: T) {}
}
```

`classes!` accepts string literals. It logs a warning for an unknown
class name and ignores it. A class that names an unknown theme token
reports the token and the nearest registered names in a debug build,
and uses a fallback colour in a release build. The class names follow the token names in
`sigil::style::Token`, for example `bg-surface`, `text-on-surface`,
and `border-border`.

### Animation

| Name | Signature | Purpose |
| --- | --- | --- |
| `Animator` | `Animator::new(signal: Signal<f32>, config: AnimationConfig)` | Animate a signal toward targets. |
| `AnimationConfig` | `enum AnimationConfig { Spring { preset }, Tween { ms, easing } }` | Spring or tween settings. |
| `SpringPreset` | `enum SpringPreset { Gentle, Smooth, Snappy, Bouncy, Stiff }` | Spring constants. |
| `Easing` | `enum Easing` | Tween easing curve. |
| `CubicBezier` | `CubicBezier::new(x1, y1, x2, y2)` | Custom easing curve. |
| `Keyframe` | `Keyframe::new(offset: f32, value: f32)` | One keyframe. |
| `Keyframes` | `Keyframes::new(frames, duration: Duration)` | A keyframe animation. |
| `AnimationDirection` | `enum AnimationDirection` | Play direction of keyframes. |
| `AnimationFill` | `enum AnimationFill { None, Forwards, Backwards, Both }` | Value before and after keyframes. |
| `Playback` | `Playback::new(value: Signal<f32>, frames: Keyframes)` | Play keyframes into a signal. |
| `MotionPrefs` | `MotionPrefs::reduced() -> bool`, `MotionPrefs::set(bool)` | Reduced-motion preference. |

```rust,no_run
use sigil::prelude::*;
use std::time::Duration;

fn tree(cx: &mut Cx) -> NodeId {
    let opacity = signal(0.0_f32);
    let _ = Animator::new(opacity, AnimationConfig::Spring { preset: SpringPreset::Snappy });
    let easing = Easing::Bezier(CubicBezier::new(0.4, 0.0, 0.2, 1.0));
    let _ = AnimationConfig::Tween { ms: 200, easing };
    let frames = Keyframes::new([Keyframe::new(0.0, 0.0), Keyframe::new(1.0, 1.0)], Duration::from_millis(300));
    let _ = (AnimationDirection::Alternate, AnimationFill::Forwards, MotionPrefs::reduced());
    let _ = Playback::new(opacity, frames);
    cx.add(container().animate(NumericAtomKind::Opacity, opacity))
}

fn main() {
    let _ = tree;
}
```

### Widgets: controls and fields

| Name | Signature | Purpose |
| --- | --- | --- |
| `button` | `fn button(label: impl Into<TextContent>) -> ButtonBuilder` | A push button. |
| `ButtonBuilder` | `primary()`, `outline()`, `ghost()`, `destructive()`, `icon_svg(svg)`, `activate(f)`, `size(Size)` | Builder of a button. `outline()` draws a framed ghost button. `icon_svg` puts an SVG icon before the label (feature `scene`). A button below the default size, or with a height of 32 px or less, uses the `text-xs` label. |
| `icon_button` | `fn icon_button(glyph, activate: impl Fn() + 'static) -> ButtonBuilder` | A button that shows one glyph. |
| `checkbox` | `fn checkbox(checked: Signal<bool>) -> CheckboxBuilder` | A checkbox. |
| `CheckboxBuilder` | `struct CheckboxBuilder`; `indeterminate(Signal<bool>)` | Builder of a checkbox. A 16 px box with an `on-accent` mark. `indeterminate` draws a dash while the signal is true. Table selection uses the same box. |
| `toggle` | `const fn toggle(on: Signal<bool>) -> ToggleBuilder` | An on/off switch. |
| `ToggleBuilder` | `struct ToggleBuilder`; `label(text)`, `accessible(name)` | Builder of a switch. `accessible` names the switch for assistive tech without visible text. The off track is `gray-6` with a `gray-11` thumb; a disabled off track is `gray-4`. |
| `radio_group` | `fn radio_group(selected: Signal<String>) -> RadioGroupBuilder` | A set of radio options; `.label()` and `.labelled_by()` name the group. |
| `segmented` | `fn segmented(active: Signal<usize>, labels: Vec<String>) -> SegmentedBuilder` | A framed row of options where one is active; `.label()` names the group and `.height()` sets its height. |
| `RadioGroupBuilder` | `struct RadioGroupBuilder` | Builder of a radio group. |
| `slider` | `fn slider(value: Signal<f32>, range: RangeInclusive<f32>) -> SliderBuilder` | A value slider. |
| `SliderBuilder` | `struct SliderBuilder`; `step(f32)`, `length(f32)`, `label(text)`, `marks(labels)`, `change(fn)` | Builder of a slider. `marks` draws one label per stop under a horizontal track, spaced evenly from the minimum to the maximum; the current stop is highlighted and a click on a label moves to it. |
| `Orientation` | `enum Orientation { Horizontal, Vertical }` | Slider axis. |
| `text_field` | `fn text_field(value: Signal<String>) -> TextFieldBuilder` | A single-line text input. |
| `TextFieldBuilder` | `struct TextFieldBuilder` | Builder of a text input. |
| `text_edit` | `fn text_edit(value: Signal<String>) -> TextEditBuilder` | A multi-line text editor. |
| `TextEditBuilder` | `struct TextEditBuilder` | Builder of a text editor. |
| `TextEditState` | `TextEditState::new(value, caret: Signal<usize>)` | Caret and undo state of a text editor. |
| `FocusCaret` | `enum FocusCaret { Keep, End, SelectAll }` | Where `TextFieldBuilder::focus_caret` puts the caret when `focus_when` focuses the field. |
| `number_field` | `fn number_field<T: NumericKind>(value: Signal<T>, range: RangeInclusive<T>) -> NumberFieldBuilder<T>` | A numeric input. |
| `NumberFieldBuilder` | `struct NumberFieldBuilder<T>` | Builder of a numeric input. |
| `NumericKind` | `trait NumericKind` | Number types that `number_field` accepts. |
| `date_field` | `fn date_field(value: Signal<Date>) -> DateFieldBuilder` | A date input. |
| `DateFieldBuilder` | `struct DateFieldBuilder` | Builder of a date input. |
| `Date` | `struct Date { year: i32, month: u8, day: u8 }` | A calendar date. |
| `color_field` | `fn color_field(value: Signal<Rgba8>) -> ColorFieldBuilder` | A colour input. |
| `ColorFieldBuilder` | `struct ColorFieldBuilder` | Builder of a colour input. |
| `field` | `fn field(label, control: impl IntoNode + 'static) -> NodeBuilder` | A label above a control. |
| `validated_field` | `fn validated_field(label, control: TextFieldBuilder, error: impl Reactive<Option<String>>) -> NodeBuilder` | A text input with an error line. |
| `label` | `fn label(source: impl Into<TextContent>) -> TextBuilder` | One-line label text. |
| `chip` | `fn chip(label, tone: Tone) -> ChipBuilder` | A small tag. |
| `badge` | `fn badge(label, tone: Tone) -> ChipBuilder` | A small status tag. |
| `ChipBuilder` | `struct ChipBuilder` | Builder of a chip or badge. |
| `interactive` | `fn interactive(inner: NodeBuilder) -> InteractiveBuilder` | Make any node hoverable, pressable, and focusable. |
| `InteractiveBuilder` | `struct InteractiveBuilder` | Builder of an interactive node. |
| `InteractionState` | `InteractionState::new(enabled: bool)` | Hover, press, and focus signals. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
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
    let checked = validated_field("Name", text_field(name), move || name.with(|n| n.is_empty().then(|| "Required".to_string())));
    let _: ChipBuilder = chip("beta", Tone::Info);
    let _: (ChipBuilder, TextBuilder) = (badge("3", Tone::Error), label("Label"));
    let _: InteractiveBuilder = interactive(container());
    let _ = InteractionState::new(true);
    let _: Option<TextFieldBuilder> = None;
    fn _numeric<T: NumericKind>() {}
    cx.add(container().child(checked).child(save).child(close))
}

fn main() {
    let _ = tree;
}
```

### Widgets: choice, collections, and documents

| Name | Signature | Purpose |
| --- | --- | --- |
| `dropdown` | `fn dropdown(selected: Signal<usize>, options: Vec<String>) -> DropdownBuilder` | A select box. |
| `DropdownBuilder` | `struct DropdownBuilder` | Builder of a select box. |
| `combobox` | `fn combobox(state: Combobox) -> NodeBuilder` | A text input with a filtered list. |
| `Combobox` | `Combobox::new(options: Vec<String>)` | State of a combobox. |
| `menu` | `fn menu(open: Signal<bool>) -> MenuBuilder` | A popup menu. |
| `MenuBuilder` | `struct MenuBuilder`; `host(owner)` | Builder of a popup menu. Item actions run under the owner current when the menu was built, or under `host`. |
| `context_menu` | `fn context_menu(target, items: impl Fn() -> Vec<MenuItem>) -> NodeBuilder` | Wraps `target`; a right click opens a menu of `items()` at the pointer. |
| `MenuItem` | `struct MenuItem { label, shortcut, disabled, submenu_indicator }` | One menu entry. |
| `menu_bar` | `fn menu_bar() -> MenuBarBuilder` | An in-window menu bar. |
| `MenuBarBuilder` | `struct MenuBarBuilder` | Builder of a menu bar. |
| `command_menu` | `fn command_menu() -> CommandMenuBuilder` | A searchable command palette. |
| `CommandMenuBuilder` | `struct CommandMenuBuilder` | Builder of a command palette. |
| `SubmenuBuilder` | `struct SubmenuBuilder` | Builder of a command palette submenu. |
| `fuzzy_score` | `fn fuzzy_score(query: &str, text: &str) -> Option<FuzzyMatch>` | Rank text against a query. |
| `FuzzyMatch` | `struct FuzzyMatch { score: usize, positions: Vec<usize> }` | Result of `fuzzy_score`. |
| `table` | `fn table<Row>(state: TableState<Row>) -> TableBuilder<Row>` | A sortable, filterable table. |
| `TableBuilder` | `struct TableBuilder<Row>`; `framed(bool)`, `row_activate(impl Fn(&Row) + 'static)` | Builder of a table. `row_activate` runs on a plain left click on a row, including its padding but not its selection box, and on Enter while the row has focus. It leaves the selection as the click set it. `framed(false)` drops the rounded outer border so the table runs edge to edge. The header fills with `table-header`; row dividers use `border-subtle`. Selected rows fill with `accent-5`, and `accent-6` under hover or press. |
| `TableState` | `TableState::new(rows: Vec<Row>, columns: Vec<TableColumn<Row>>)` | Rows, columns, sort, and selection. |
| `TableColumn` | `TableColumn::new(id: &'static str, title)` | One table column. |
| `ColWidth` | `enum ColWidth { Fixed(f32), Fraction(f32) }` | Column width. |
| `ColAlign` | `enum ColAlign { Start, End }` | Column alignment. |
| `Density` | `enum Density { Grid, Compact, Default }` | Row density of a table. |
| `Identifiable` | `trait Identifiable { type Id; fn id(&self) -> Self::Id; }` | Stable row identity. |
| `virtual_list` | `fn virtual_list<T, R, I>(items: I, render: R) -> VirtualListBuilder<T, R>` | A list that builds only the visible rows. |
| `VirtualListBuilder` | `struct VirtualListBuilder<T, R>`; `measured()`, `estimate(len)`, `scroll_bind(Signal<f32>)`, `follow(Signal<usize>)`, `content_height_bind(Signal<f32>)`, `max_scroll_bind(Signal<f32>)` | Builder of a virtual list. `measured()` lays rows out at the list width and caches their heights. The two extent binds receive the content height and the largest scroll offset. |
| `ScrollAnchor` | `enum ScrollAnchor { Top, Bottom, Index(usize) }` | Where a virtual list stays scrolled. |
| `SelectionMode` | `enum SelectionMode { None, Single, Multi, Extend }` | Selection mode of a list. |
| `SelectionState` | `SelectionState::new(mode: SelectionMode)` | Selected indexes of a list. |
| `list_row` | `fn list_row(selected: bool, title, detail) -> NodeBuilder` | One row of a list. |
| `list_row_with` | `fn list_row_with(selected: bool, title, slots: ListRowSlots) -> NodeBuilder` | A list row with a marker, status, and columns. |
| `ListRowSlots` | `struct ListRowSlots { depth, disclosure, marker, status, columns, shrink, title_width, tone }` | Optional parts of a list row. `title_width` gives the title a fixed width so columns line up after it. |
| `ListColumn` | `ListColumn::new(text, width)`; `start()`; `From<(TextContent, f32)>` | One fixed-width column of a list row. Columns align to the end unless `start()` is set. |
| `RowTone` | `enum RowTone { Normal, Muted, Header }` | Text tone of a list row. `Muted` dims an unavailable row. `Header` labels columns and takes no hover or selection. |
| `Disclosure` | `enum Disclosure { None, Leaf, Collapsed, Expanded }` | The tree chevron slot of a list row. |
| `kv_grid` | `fn kv_grid(rows: &[(&'static str, &'static str)]) -> NodeBuilder` | A two-column key/value grid. |
| `kv_row` | `fn kv_row(label, value: impl IntoNode + 'static) -> NodeBuilder` | One key/value row. |
| `tabs` | `fn tabs(active: Signal<usize>, labels: Vec<String>) -> TabsBuilder` | A tab strip. |
| `TabsBuilder` | `struct TabsBuilder` | Builder of a tab strip. |
| `code_edit` | `fn code_edit(lines: Signal<Vec<String>>, caret: Signal<(usize, usize)>) -> CodeEditBuilder` | A code editor with built-in `text_edit` editing. |
| `CodeEditBuilder` | `struct CodeEditBuilder` | Builder of a code editor. |
| `HlSpan` | `struct HlSpan { start: usize, end: usize, token: &'static str }` | A highlight span in a code line. |
| `highlight` | `fn highlight(language: &str, source: &str) -> Vec<Vec<Highlight>>` | Syntax marks per line for rust, sh, toml, json, python, js/ts, and diff. Other languages get no marks. |
| `Highlight` | `struct Highlight { start: usize, end: usize, token: Token }` | One syntax mark: a byte range and its theme token. |
| `hl_spans` | `fn hl_spans(language: &str, source: &str) -> Vec<Vec<HlSpan>>` | `highlight` as column spans for `code_edit(..).highlights(..)`. |
| `markdown` | `fn markdown(source: Signal<String>) -> NodeBuilder` | Rendered markdown. |
| `markdown_inline` | `fn markdown_inline(source: &str) -> TextBuilder` | One wrapped paragraph with inline markdown spans. |
| `markdown_edit` | `fn markdown_edit(source: Signal<String>) -> MarkdownEditBuilder` | A markdown editor with preview. |
| `MarkdownEditBuilder` | `struct MarkdownEditBuilder` | Builder of a markdown editor. |
| `html` | `fn html(source: Signal<String>) -> NodeBuilder` | Rendered HTML subset. |
| `mermaid` | `fn mermaid(source: impl Into<String>) -> NodeBuilder` | A rendered mermaid diagram. |
| `diff` | `fn diff(patch: impl Into<String>) -> DiffBuilder` | A rendered unified diff. |
| `DiffBuilder` | `struct DiffBuilder` | Builder of a diff view. |
| `DiffLayout` | `enum DiffLayout { Split, Unified }` | Diff layout. |
| `terminal` | `fn terminal(lines: Signal<Vec<String>>, on_input: impl Fn(String) + 'static) -> NodeBuilder` | A terminal transcript with an input line. |

```rust,no_run
use sigil::prelude::*;

#[derive(Clone)]
struct File { name: String, size: u64 }

fn tree(cx: &mut Cx) -> NodeId {
    let _: DropdownBuilder = dropdown(signal(0), vec!["One".into(), "Two".into()]);
    let _ = combobox(Combobox::new(vec!["red".into(), "blue".into()]));
    let _: MenuBuilder = menu(signal(false));
    let _: Option<(MenuItem, SubmenuBuilder)> = None;
    let _: (MenuBarBuilder, CommandMenuBuilder) = (menu_bar(), command_menu());
    let _: Option<FuzzyMatch> = fuzzy_score("sv", "save file");
    let size = TableColumn::new("size", "Size").numeric().width(ColWidth::Fixed(80.0)).align(ColAlign::End);
    let columns = vec![TableColumn::new("name", "Name").cell(|f: &File| label(f.name.clone())), size];
    let files: TableBuilder<File> = table(TableState::new(vec![File { name: "a".into(), size: 1 }], columns));
    let _ = (Density::Compact, ScrollAnchor::Bottom, SelectionState::new(SelectionMode::Single));
    let list: VirtualListBuilder<String, _> = virtual_list(vec!["a".to_string()], |_, s: &String| list_row(false, s.clone(), ""));
    let (scroll, max_scroll) = (signal(0.0_f32), signal(0.0_f32));
    let list = list.measured().estimate(rows(2.0)).scroll_bind(scroll).max_scroll_bind(max_scroll);
    let list = list.content_height_bind(signal(0.0)).follow(signal(0));
    let _ = list_row_with(true, "Inbox", ListRowSlots { marker: Some("*".into()), ..ListRowSlots::default() });
    let _ = (kv_grid(&[("Name", "Sigil")]), kv_row("Size", text("1 KB")));
    let _: TabsBuilder = tabs(signal(0), vec!["A".into(), "B".into()]);
    let _: CodeEditBuilder = code_edit(signal(vec!["fn main() {}".into()]), signal((0, 0)));
    let _ = HlSpan { start: 0, end: 2, token: "keyword" };
    let _: Vec<Vec<Highlight>> = highlight("rust", "fn main() {}");
    let _ = code_edit(signal(vec!["let x = 1;".into()]), signal((0, 0))).highlights(signal(hl_spans("rust", "let x = 1;")));
    let doc = signal("# Title".to_string());
    let _ = (markdown(doc), markdown_edit(doc), html(signal("<b>x</b>".into())), mermaid("graph TD; A-->B"));
    let _: DiffBuilder = diff("--- a\n+++ b\n").layout(DiffLayout::Unified);
    let _ = terminal(signal(Vec::new()), |line| println!("{line}"));
    fn _id<T: Identifiable>() {}
    cx.add(container().child(files).child(list))
}

fn main() {
    let _ = tree;
}
```

### Widgets: frame, overlays, and display

| Name | Signature | Purpose |
| --- | --- | --- |
| `app_shell` | `fn app_shell(top, work, status, modal: Option<impl IntoNode>) -> NodeBuilder` | Top bar, work area, status bar, and modal slot. |
| `sidebar` | `fn sidebar(child: impl IntoNode + 'static) -> NodeBuilder` | A side column. |
| `sidebar_width` | `fn sidebar_width(width: impl IntoPx, child) -> NodeBuilder` | A side column of a set width. |
| `status_bar` | `fn status_bar<I: IntoIterator>(children: I) -> NodeBuilder` | A status line. |
| `panel` | `fn panel() -> PanelBuilder` | A titled panel. |
| `PanelBuilder` | `struct PanelBuilder` | Builder of a panel. |
| `card` | `fn card(child: impl IntoNode + 'static) -> NodeBuilder` | A bordered card. |
| `sheet` | `fn sheet(show: Signal<bool>) -> ModalBuilder` | A panel that slides in from a side. |
| `modal` | `fn modal(show: Signal<bool>) -> ModalBuilder` | A modal dialog. |
| `ModalBuilder` | `struct ModalBuilder`; `close_button(bool)` | Builder of a modal or sheet. A titled dialog shows a close button that sets `show` to false; `close_button(false)` removes it. Children scroll in a body between the title and the footer, so the title and actions stay visible when the dialog reaches its maximum height. |
| `Side` | `enum Side { Left, Right, Bottom }` | Edge of a sheet. |
| `splitter` | `const fn splitter(orientation: SplitterOrientation) -> SplitterBuilder` | Resizable panes. |
| `SplitterBuilder` | `struct SplitterBuilder` | Builder of a splitter. |
| `SplitterOrientation` | `enum SplitterOrientation { Horizontal, Vertical }` | Splitter axis. |
| `scroll` | `fn scroll(content: NodeBuilder, extent: impl IntoScrollExtent) -> ScrollBuilder` | A scroll area with a known extent. |
| `scroll_auto` | `fn scroll_auto(content: NodeBuilder) -> ScrollBuilder` | A scroll area sized by its content. |
| `ScrollBuilder` | `struct ScrollBuilder`; `fill()`, `fit()` | Builder of a scroll area. `fill()` takes the parent's height. `fit()` takes the content height up to the parent's height, then scrolls. |
| `section_label` | `fn section_label(title) -> NodeBuilder` | A section heading row. |
| `hint_bar` | `fn hint_bar(keys, right) -> NodeBuilder` | A key hint row. |
| `hint_overlay` | `fn hint_overlay(hints: &[(&str, &str)]) -> HintOverlayBuilder` | An overlay that lists key bindings. |
| `HintOverlayBuilder` | `struct HintOverlayBuilder` | Builder of a key hint overlay. |
| `notice_band` | `fn notice_band(body, tone: Tone) -> NodeBuilder` | A full-width notice. |
| `empty_state` | `fn empty_state(title, body) -> NodeBuilder` | A placeholder for empty content. |
| `spacer` | `fn spacer() -> NodeBuilder` | Flexible empty space. |
| `hairline` | `fn hairline() -> NodeBuilder` | A one-pixel rule. |
| `divider_horizontal` | `fn divider_horizontal() -> NodeBuilder` | A horizontal divider. |
| `divider_vertical` | `fn divider_vertical() -> NodeBuilder` | A vertical divider. |
| `surface` | `fn surface(key: u64) -> NodeBuilder` | A rectangle that the embed host draws. |
| `tooltip` | `fn tooltip(label: impl Into<String>) -> TooltipBuilder` | A hover tooltip. |
| `TooltipBuilder` | `struct TooltipBuilder` | Builder of a tooltip. |
| `toast` | `fn toast(message: Signal<Option<String>>) -> NodeBuilder` | One transient message. |
| `toast_queue` | `fn toast_queue(queue: ToastQueue) -> NodeBuilder` | A stack of transient messages. |
| `ToastQueue` | `ToastQueue::new()`; `info`, `success`, `warning`, `error` | Queue of toast messages. |
| `ToastEntry` | `struct ToastEntry { level: ToastLevel, message: String }` | One queued toast. |
| `ToastLevel` | `enum ToastLevel { Info, Success, Warning, Error }` | Toast severity. |
| `progress` | `fn progress(value: Signal<f32>, max: f32) -> ProgressBuilder` | A progress bar; `.label()` names it, else "Progress". |
| `ProgressBuilder` | `struct ProgressBuilder` | Builder of a progress bar. |
| `gauge` | `fn gauge(value: Signal<f32>) -> GaugeBuilder` | A radial gauge. |
| `GaugeBuilder` | `struct GaugeBuilder` | Builder of a gauge. |
| `chart` | `fn chart(values: impl IntoIterator<Item = f32>) -> ChartBuilder` | A bar or line chart. |
| `ChartBuilder` | `bar()`, `line()`, `labels(..)`, `series(..)` | Builder of a chart. |
| `ChartMark` | `enum ChartMark { Bar, Line }` | Chart mark. |
| `heatmap` | `fn heatmap<F: Fn(usize, usize) -> Rgba8>(grid_w, grid_h, color: F) -> NodeBuilder` | A grid of coloured cells. |
| `dot` | `fn dot(tone: Tone) -> DotBuilder` | A status dot. |
| `DotBuilder` | `struct DotBuilder` | Builder of a status dot. |
| `avatar` | `fn avatar(initials) -> NodeBuilder` | Initials in a circle. |
| `spinner` | `fn spinner() -> NodeBuilder` | A busy indicator named "Loading". |
| `spinner_frame` | `fn spinner_frame(tick: u64) -> &'static str` | The spinner glyph for a tick count. |
| `icon` | `fn icon(glyph: impl Into<TextContent>) -> IconBuilder` | An icon glyph. |
| `icon_svg` | `fn icon_svg(svg: impl Into<Rc<str>>) -> IconBuilder` | An SVG icon. Its colour comes from the tone and the active theme at paint time. |
| `IconBuilder` | `tone(t)`, `size(s)`, `px(px)`, `strong()` | Builder of an icon. `px` sets an exact icon size. `strong` uses the high-contrast step of the tone scale. |
| `path` | `const fn path(width: u32, height: u32) -> PathNode` | A vector path. |
| `PathNode` | `move_to`, `line_to`, `quad_to`, `cubic_to`, `close`, `fill`, `stroke` | Builder of a vector path. |
| `PathCmd` | `enum PathCmd { MoveTo, LineTo, QuadTo, CubicTo, Close }` | One path command. |
| `PathFill` | `enum PathFill { Solid, Theme, Linear, Radial }` | Path fill paint. `Theme(Token)` resolves at paint time. |
| `PathColor` | `enum PathColor { Rgba(Rgba8), Token(Token) }` | Colour of `fill` and `stroke`. A `Token` follows the active theme. |
| `image_rgba` | `fn image_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<ImageBuilder>` | An image from RGBA bytes. |
| `image_bytes` | `fn image_bytes(encoded: &[u8]) -> Option<ImageBuilder>` | An image from PNG or JPEG bytes (feature `image-decode`). |
| `patterns` | `mod patterns` | The module of the frame and row patterns. |

```rust,no_run
use sigil::prelude::*;

fn tree(cx: &mut Cx) -> NodeId {
    let open = signal(false);
    let queue = ToastQueue::new();
    queue.info("Saved");
    let _: Option<ToastEntry> = None;
    let dialog: ModalBuilder = modal(open);
    let _: ModalBuilder = sheet(open);
    let _ = Side::Right;
    let _: SplitterBuilder = splitter(SplitterOrientation::Horizontal);
    let _: ScrollBuilder = scroll(container(), 400.0_f32);
    let _: PanelBuilder = panel();
    let _: HintOverlayBuilder = hint_overlay(&[("q", "quit")]);
    let _ = (section_label("Files"), hint_bar("q quit", "v0.1"), notice_band("Offline", Tone::Warn));
    let _ = (empty_state("No files", "Add one"), hairline(), divider_horizontal(), divider_vertical());
    let _ = (surface(1), toast(signal(None)), toast_queue(queue), ToastLevel::Error);
    let _: TooltipBuilder = tooltip("Help");
    let _: (ProgressBuilder, GaugeBuilder) = (progress(signal(0.3), 1.0), gauge(signal(0.5)));
    let _: ChartBuilder = chart([1.0, 2.0, 3.0]).line();
    let _ = (ChartMark::Bar, PathCmd::Close, PathFill::Solid(Rgba8 { r: 0, g: 0, b: 0, a: 255 }));
    let _ = heatmap(4, 4, |x, y| Rgba8 { r: (x * 60) as u8, g: (y * 60) as u8, b: 0, a: 255 });
    let _: DotBuilder = dot(Tone::Success);
    let _ = (avatar("TG"), spinner(), spinner_frame(3));
    let _: IconBuilder = icon("*");
    let _: IconBuilder = icon_svg(sigil::icons::builtin::GEAR_SVG).tone(Tone::Primary);
    let _: PathNode = path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0).stroke(PathColor::Token(sigil::style::Token::Gray11), 1.5);
    let _: PathNode = path(16, 16).move_to(0.0, 0.0).line_to(16.0, 16.0).close().fill(Rgba8 { r: 0, g: 0, b: 0, a: 255 });
    let _: Option<ImageBuilder> = image_rgba(1, 1, vec![0, 0, 0, 255]);
    let _ = patterns::spacer();
    let work = sidebar(sidebar_width(200.0, card(text("Card"))));
    cx.add(app_shell(text("Top"), work, status_bar([text("Ready")]), Some(dialog)))
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
| `DRAG_THRESHOLD_PX` | `const DRAG_THRESHOLD_PX: f32 = 4.0` | Pointer travel before a drag starts. |
| `DROP_TARGET_CLASS` | `const DROP_TARGET_CLASS: &str` | Class on a drop target under the pointer. |

```rust,no_run
use sigil::prelude::*;
use sigil::widgets::Axis;

fn tree(cx: &mut Cx) -> NodeId {
    let drag: DragState<u32> = DragState::new();
    let source = container().drag_source(drag.clone(), 7);
    let target = container().drop_target(drag.clone(), |n: &u32| *n > 0, |n| println!("{n}"));
    let ghost = drag_ghost(drag, |n: &u32| text(n.to_string()));
    let width = signal(200.0_f32);
    let handle = axis_drag(container(), Axis::X, move || width.peek(), move |start, dx| width.set(start + dx), || {});
    let _ = (DRAG_THRESHOLD_PX, DROP_TARGET_CLASS);
    cx.add(container().child(source).child(target).child(ghost).child(handle))
}

fn main() {
    let _ = tree;
}
```

### Clipboard

| Name | Signature | Purpose |
| --- | --- | --- |
| `copy` | `fn copy(text: &str) -> AppResult<Delivery>` | Write text to the installed clipboard. |
| `paste` | `fn paste() -> AppResult<String>` | Read text from the installed clipboard. |
| `install` | `fn install(clip: impl Clipboard + 'static)` | Replace the installed clipboard. |
| `Clipboard` | `trait Clipboard { fn set(&self, &str) -> Result<Delivery, ClipError>; fn get(&self) -> Option<String>; }` | A clipboard back end. |
| `Delivery` | `enum Delivery { Confirmed, Requested }` | Whether the OS took the text. |
| `ClipError` | `ClipError::new(msg)` | A clipboard back end failed. |
| `OsClipboard` | `struct OsClipboard` | The OS clipboard (feature `clipboard`). |

```rust,no_run
use sigil::prelude::*;

struct Memory;

impl Clipboard for Memory {
    fn set(&self, _text: &str) -> Result<Delivery, ClipError> { Err(ClipError::new("read-only")) }
    fn get(&self) -> Option<String> { Some("text".into()) }
}

fn main() -> AppResult<()> {
    install(Memory);
    let _: Delivery = copy("hello")?;
    let _: String = paste()?;
    let _: Option<OsClipboard> = None;
    Ok(())
}
```

## Jobs to symbols

| Job | Symbol |
| --- | --- |
| Import the application surface | `use sigil::prelude::*;` |
| Open a window and run the frame loop | `sigil::run(config, tree)` (feature `desktop`) |
| Run the tree in a terminal | `sigil::run(AppConfig::new(title).terminal(), tree)` (feature `terminal`) |
| Run the tree in a browser | `sigil::run(config, tree)` built for `wasm32-unknown-unknown` ([Web host](guide/web.md)) |
| Draw into an existing `<canvas>` | `AppConfig::canvas(id)` (browser host; native hosts ignore it) |
| Open a file or URL with the operating system handler | `sigil::open(target)` (feature `desktop`) |
| Stop the application | `sigil::request_exit()`; in the terminal host it also works from a `run_with` hook |
| Send a window command | `request_window_command`, `WindowCommand` |
| Embed the tree in another engine | `Runtime::scope`, `Host::new`, `Host::build` or `Host::mount` |
| Route an input event to an embedded tree | `Host::route(&mut EventRouter, ShellEvent)` |
| Run an accessibility action on an embedded tree | `Host::perform(&mut EventRouter, NodeId, &accesskit::ActionRequest)` |
| Lay out and paint an embedded tree | `Host::layout`, then `Host::emit(viewport)` for the `CommandScene`; `Host::scene(viewport)` does both |
| Draw a scene onto a cell grid | `sigil::terminal::raster(&scene, host.theme(), cols, rows, cell)` (feature `terminal`) |
| Get the rectangles of host-drawn surfaces | `surface(key)` in the tree, then `Host::surfaces` |
| Get the accessibility tree of an embedded tree | `Host::accessibility_tree` |
| Pump external events into the terminal loop, or run code after each frame | `sigil::terminal::run_with(config, tree, &mut Hooks)` |
| Run `$EDITOR` or another child process with the terminal restored | `sigil::terminal::suspend(f) -> io::Result<R>` |
| Set the terminal window title | `sigil::terminal::set_title(title) -> io::Result<()>` |
| Ring the terminal bell, with an optional desktop notification | `sigil::terminal::bell(note: Option<&str>) -> io::Result<()>` |
| Check whether the terminal window has focus | `sigil::terminal::focused() -> bool` |
| Send Ctrl+C to the tree instead of exiting | `AppConfig::quit_on_ctrl_c(false)` |
| Set the terminal frame interval (default 33 ms) | `AppConfig::frame_interval(Duration)` |
| Turn terminal mouse capture on or off | `sigil::terminal::set_mouse_capture(yes) -> io::Result<()>` |
| Render one terminal frame to a string | `sigil::terminal::present(tree, cols, rows, events)` |
| Drive the terminal host one step at a time | `sigil::terminal::step`, `Host::cells` |
| Detect the terminal graphics protocol | `sigil::terminal::detect`, `sigil::terminal::Protocol` |
| Bind a key on a node | `.key(shortcut, handler)` from `NodeEventsExt` |
| Bind a key that fires from anywhere | `.shortcut(shortcut, handler)` from `NodeEventsExt` |
| See every key before the focused descendant | `.key_capture_any(handler)`; call `event.stop_propagation()` to keep the key from the target |
| Run a callback on a fixed period | `interval(Duration, f) -> IntervalId`, `sigil::reactive::cancel_interval` |
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
| `Rect` with `CellBorder` | Box-drawing glyphs on each side whose width is above zero. A rounded border uses rounded corners. |
| `GlyphRun` | Text cells with bold and italic. `TextFlow` selects one line, wrap, or ellipsis. |
| `Image` | The detected graphics protocol, or sextant glyphs. The crop and the clip apply. |
| `PushClip`, `PushLayer` | Clip the commands inside. |
| `SaveLayer`, `PushLayer` opacity | Blend fills, borders, and text against the cell below, with the active theme. |
| Shadow, gradient, pattern, sprite, decoration | Not drawn. |

TopLayer content comes last in the scene, so it draws above clipped
content.

## Errors

| Type | Returned by | Meaning |
| --- | --- | --- |
| `sigil::Error` | `sigil::run`, `sigil::terminal::run_with` | Opaque application error. `Error::kind` returns an `ErrorKind` (`Window`, `Gpu`, `Font`, `Io`, `Task`, `Config`, `Other`). `Error::new` wraps any `std::error::Error` as `Other`; `Error::with_kind` sets the kind. `Error::source_error` returns the inner error. |
| `ScheduleError` | `SignalWriter::set`, `SignalWriter::update` | `Closed`: the dispatcher is closed. `Full`: the dispatcher queue is full. |
| `LayoutError` | `Host::layout`, `Host::scene`, `sigil::terminal::present`, `sigil::terminal::step` | The layout pass failed. It converts into `sigil::Error`. |
| `sigil_sign::SignError` | `sign_file`, `verify_file` | `KeyFile`, `Key`, `Package`, `Envelope`, or `TrustedComment`: the key, the package, the signature, or the signed file name or version failed. |
| `sigil_sign::PackError` | `macos_app`, `windows_dir` | `Missing`, `Io`, `Codesign`, `Escape`, `Symlink`, `Occupied`, or `NotFile`. |

## Inspection

With the `inspect` cargo feature, an app exposes a local control socket
when `SIGIL_INSPECT` is set. `SIGIL_INSPECT=1` binds `127.0.0.1:5761`;
any other value is an address to bind. The socket speaks one JSON
object per line, one reply per request.

| `op` | Request | Reply |
| --- | --- | --- |
| `tree` | – | `nodes`: id to `{role, name, description, bounds, children, focused, selected, expanded, toggled, value, numeric}` plus `root` and `focus`. Bounds are in points on the window host and cells on the terminal host. |
| `snapshot` | – | `text`: the accessibility text dump. |
| `screenshot` | – | Terminal: `text` with the painted grid. Window: `png` (base64) plus `width`/`height` in physical pixels. |
| `surfaces` | – | `surfaces`: keyed surface holes and rects. |
| `settle` | – | `ok` after servicing; `settled` is false while the frame stays dirty or animations run. |
| `click` | `col`/`row` (terminal cells) or `x`/`y` (points), `button` | Moves, presses, and releases the pointer through the normal router. |
| `move`, `down`, `up` | same coordinates | One pointer phase. |
| `scroll` | coordinates plus `dx`/`dy` in lines | A scroll event. |
| `key` | `key` (a `NamedKey` name or one character), `ctrl`/`shift`/`alt`/`meta` | Key down and up. |
| `type` | `text` | Key events per character. |
| `profile` | `last` (frame count, default 30) | `enabled`, `frames` (index, cause, `wall_ms`, per-phase ms, counters), and `summary` (the exit report text). |
| `profile_dump` | `path` (optional) | Writes the Chrome trace JSON now and replies with `path`. Defaults to `SIGIL_PROFILE`'s value or `sigil-profile.json`. |
| `profile_on`, `profile_off` | – | Enable or stop frame collection without restarting. |
| `hud` | `on` (default true) | Draws the frame-HUD overlay: a sparkline and last-frame phase readout top-right. Terminal host draws cells; the desktop host overlays scene commands at paint. Enabling turns profiling on. |

Requests are serviced between frames on the UI thread, so replies
reflect the tree as last laid out. The socket is loopback by
convention; the feature is off without the environment variable.

## Conventions

- Make reactive writes on the thread that owns the runtime. From
  another thread, use `signal_writer`, `post`, or `dispatch_to_owner`.
- `sigil::run` installs the OS clipboard on the window host. The
  terminal host installs an OSC 52 clipboard. An embed host calls
  `install(OsClipboard)` itself.
- Public event, command, error, and config types are
  `#[non_exhaustive]`. Match them with a `_` arm. Build `AppConfig`
  with its builder methods.
- The renderer API takes `wgpu` types. `sigil::wgpu` re-exports the
  `wgpu` version that `sigil-ui` uses.
- `SIGIL_BACKEND` accepts `auto`, `wgpu`, and `cpu`. See
  [Troubleshooting](guide/troubleshooting.md).
- Parsers for markdown, HTML, SVG, and theme files run on the UI
  thread. See `SECURITY.md` for the input surfaces.
