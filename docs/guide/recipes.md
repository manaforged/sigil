# Recipes

Each recipe is a complete program or tree function for one job. Every code block on this page compiles as a doctest of the `sigil` crate. The [API reference](https://docs.rs/sigil-ui) lists the signature of each item used here.

## Keys

A key event goes to the focused node first. Handlers run in two passes along the path from the root to the focused node:

1. Capture handlers (`key_capture`, `key_capture_any`) run from the root down to the focused node.
2. Bubble handlers (`key`, `key_any`) run from the focused node up to the root.

A handler calls `event.stop_propagation()` to end both passes.

Tab and Shift+Tab move focus between the capture pass and the bubble pass. A capture handler that stops propagation keeps focus in place, so `.key_capture(NamedKey::Tab, f)` on an ancestor owns Tab for its subtree.

A `Shortcut` is a key plus modifiers:

| Constructor | Matches |
| --- | --- |
| `Shortcut::plain(NamedKey::Enter)` | Enter with no modifiers |
| `Shortcut::plain('j')` | The `j` key with no modifiers |
| `Shortcut::ctrl("s")` | Ctrl+S |
| `Shortcut::primary('k')` | Cmd+K on macOS, Ctrl+K elsewhere |
| `Shortcut::new(key, Modifiers::CTRL \| Modifiers::SHIFT)` | Any modifier set |

`NamedKey::Escape`, `&str`, and `char` convert into `Key` and `Shortcut` directly, so `.key(NamedKey::Escape, f)`, `Shortcut::primary('k')`, and `event.key() == '?'` work without `Key::character(...)`. Use `.key_any(|event| ...)` on a focused node to handle any key in the bubble pass.

### Bind a key on a focused node

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let count = signal(0_i32);
    container()
        .focusable(true)
        .autofocus()
        .key(Key::character("j"), move |_| count.update(|n| *n += 1))
        .key(Key::character("k"), move |_| count.update(|n| *n -= 1))
        .key(NamedKey::Escape, |_| sigil::exit())
        .child(text(memo(move || format!("row {}", count.get()))))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("keys").terminal(), tree)
}
# }
# fn main() {}
```

`NodeId::focus` gives the node keyboard focus. Without focus, a node receives no key events through the focus path.

### Bind a key that fires from anywhere

`.shortcut(s, handler)` registers `s` as a global shortcut. The router runs the handler for the key, wherever the focus is. One node can hold many shortcuts. A plain character shortcut does not fire while a text input has focus, so typing `q` into a field does not quit.

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let name = signal(String::new());
    container()
        .shortcut(Key::character("q"), |_| sigil::exit())
        .shortcut(Shortcut::ctrl(Key::character("n")), move |_| name.set(String::new()))
        .child(text_field(name))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("global keys").terminal(), tree)
}
# }
# fn main() {}
```

### See every key before the focused widget

`key_capture_any` on an ancestor sees each key before the focused descendant. This example turns Tab inside a text editor into two spaces.

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let body = signal(String::new());
    container().child(text_field(body).multiline()).key_capture_any(move |event: &mut KeyEvent| {
        if event.key() == Key::named(NamedKey::Tab) {
            body.update(|text| text.push_str("  "));
            event.stop_propagation();
        }
    })
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("capture"), tree)
}
```

`key_capture(shortcut, f)` is the same, but runs only for one shortcut.

### Keys on a widget builder

Every widget builder implements `Widget`, so it takes the event methods. The handler goes on the root node of the widget and runs after the handlers of the widget. If the widget stops a key that you need, use `key_capture` instead. To use a `NodeBuilder` method that is not on `Widget`, such as `added`, call `.map_node(|node| node.added(f))`.

### Helper return types

A helper that returns `impl IntoNode` keeps only `IntoNode`. Chained builder methods are not available on the result. Return the concrete builder type (for example `sigil::widgets::ButtonBuilder`) or `NodeBuilder` to keep them. Under Rust 2024, `impl IntoNode` captures every lifetime in the arguments. A helper that takes `&state` and returns `impl IntoNode` then borrows `state` for the life of the node. Write `-> impl IntoNode + use<>` when the result does not use the reference, or return a concrete type.

## Theme switching

`Theme::light()`, `Theme::dark()`, and `Theme::console_dark()` build the default themes. `Theme::apply` replaces the active theme at the next frame, on the window and on the terminal host.

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let dark = signal(false);
    container().child(toggle(dark)).child(button("Apply").activate(move || {
        (if dark.get() { Theme::dark() } else { Theme::light() }).apply();
    }))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("themes"), tree)
}
```

A theme file is TOML. `base` selects `light`, `dark`, or `console_dark`. The `[tokens]` table sets token colors as `#rrggbb` or `#rrggbbaa`. `Theme: FromStr` returns `Err` if the document is not valid.

A theme file also carries the language settings. `direction` is `"ltr"` or `"rtl"`. `locale = "en"`, `"de"`, or `"ar"` selects a built-in locale; a `[locale]` table sets fields one by one: `preset` (the built-in locale to start from), `decimal`, `group` (`""` for no grouping), `zero` and `date_separator` as single characters, `group_size` from 1 to 255, `date_order` as `"ymd"`, `"dmy"`, or `"mdy"`, and `hour12` as `true` for a 12-hour clock in `time_field`. The `[strings]` table replaces built-in labels by their snake_case name, for example `close = "Schließen"`. An unknown name or value makes `Theme: FromStr` return `Err`.

```toml
base = "light"
direction = "rtl"

[tokens]

[locale]
preset = "ar"
date_order = "ymd"

[strings]
close = "إغلاق"
```

```rust,no_run
use sigil::prelude::*;

const SOURCE: &str = r##"
base = "dark"

[tokens]
accent = "#3c59e7"
surface = "#101014"
"##;

fn tree() -> impl IntoNode {
    if let Ok(theme) = SOURCE.parse::<Theme>() {
        theme.apply();
    }
    text("Custom theme")
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("theme file"), tree)
}
```

To reload a theme file while the app runs, call `watch_theme`. The watch stops when the `ThemeWatcher` is dropped, so keep it for the life of the tree. `AppConfig::theme_file(path)` loads and watches a file without code in the tree.

```rust,no_run
use sigil::app::watch_theme;
use sigil::prelude::*;
use sigil::reactive::cleanup;

fn tree() -> impl IntoNode {
    match watch_theme("theme.toml") {
        Ok(watcher) => cleanup(move || drop(watcher)),
        Err(err) => eprintln!("theme watch failed: {err}"),
    }
    text("Edit theme.toml")
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("watch").theme_file("base.toml"), tree)
}
```

`Token` names each theme color. The `Paint` methods `background`, `text_color`, and `border_color` take a token, so a misspelled token does not compile.

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    container()
        .background(Token::SurfaceSubtle)
        .border_width(1)
        .border_color(Token::Border)
        .child(text("Panel").text_color(Token::OnSurfaceMuted))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("tokens"), tree)
}
```

## Layout reference

A container function makes the box of one node. Chain setters on it:

| Container | Box |
| --- | --- |
| `column()` | Children top to bottom, with the theme gap |
| `row()` | Children left to right, centred across the row, with the theme gap |
| `grid(columns)` | Children in grid columns |
| `container()` | Block flow |

`.gap(Space::None)` removes the theme gap. `.align_items(AlignItems::Stretch)` makes the children of a row as high as the row. Every widget takes the same setters:

| Setter | Effect |
| --- | --- |
| `width`, `height` | Fixed size |
| `min_width`, `min_height`, `max_width`, `max_height` | Size limits |
| `fill_width`, `fill_height`, `fill` | 100% of the parent |
| `gap`, `gap_x`, `gap_y`, `padding`, `padding_x`, `padding_y`, `padding_top`, ..., `margin`, `margin_x`, `margin_y`, `margin_top`, ... | Spacing |
| `grow`, `shrink`, `flex_basis` | Flex sizing |
| `align_items`, `align_self`, `justify_content` | Alignment |
| `grid_columns`, `grid_rows`, `grid_column_span`, `grid_area` | Grid placement |
| `position`, `inset`, `inset_top`, ... | Absolute placement |
| `overflow`, `overflow_x`, `overflow_y` | Clip and scroll |

A size includes the padding and the border of the node. A plain number is logical pixels. `rows(n)`, `cols(n)`, and `em(n)` are text-relative. In the terminal host, `rows(1.0)` is one cell high and `cols(1.0)` is one cell wide. In a window, `rows(1.0)` is one line height and `cols(1.0)` is one character advance. Use text-relative units in a tree that runs on both hosts. See [Units](units.md).

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::layout::TrackSize;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let header = row()
        .height(rows(1.0))
        .gap(cols(1.0))
        .fill_width()
        .child(text("Title"))
        .child(spacer())
        .child(text("v0.1"));
    let names = grid([TrackSize::Px(160.0), TrackSize::Fr(1.0)])
        .gap(rows(1.0))
        .child(text("Name"))
        .child(text("Sigil"));
    let body = column()
        .grow(1.0)
        .padding(cols(2.0))
        .max_width(em(40.0))
        .child(names);
    column().fill().gap(Space::None).child(header).child(body)
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("layout").terminal(), tree)
}
# }
# fn main() {}
```

`viewport_size()` returns the viewport as a signal, in logical px on every host. On the terminal one cell is `Host::CELL` px, so divide by it to get columns and rows. Read it in a `derived` or a `children_reactive` closure to adapt the tree to the size.

```rust,no_run
use sigil::app::is_grid;
use sigil::app::viewport_size;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let size = viewport_size();
    let wide = memo(move || size.get().width > if is_grid() { 100.0 } else { 900.0 });
    text(memo(move || if wide.get() { "two columns".to_string() } else { "one column".to_string() }))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("viewport"), tree)
}
```

## Lists

`for_each(items, key, render)` calls `render` one time for each new key. A write to `items` adds, removes, and reorders rows. It does not call `render` again for a key that stays in the list.

Put each field that changes in a signal inside the item, and read that signal in the row. A field that is a plain value is read one time, when the row is built.

```rust,no_run
use sigil::prelude::*;

#[derive(Clone)]
struct Thread {
    id: u32,
    title: Signal<String>,
    unread: Signal<bool>,
}

fn inbox(threads: Signal<Vec<Thread>>) -> impl IntoNode {
    let unread = memo(move || {
        threads.with(|all| all.iter().filter(|thread| thread.unread.get()).count())
    });
    column()
        .child(text(move || format!("{} unread", unread.get())))
        .child(scroll(for_each(threads, |thread: &Thread| thread.id, thread_row)))
}

fn thread_row(thread: &Thread) -> NodeBuilder {
    let unread = thread.unread;
    row()
        .child(text(thread.title))
        .child(button("Read").activate(move || unread.set(false)))
}
```

`unread.set(false)` runs the count again, then the label that reads the count. It builds no row.

### Heterogeneous children and `switch`

Every widget builder implements `Into<NodeBuilder>`, so `.children(iter)` (and `status_bar([...])`) accepts mixed widgets via `.into()`. Use `switch(key).case(v, || ...).otherwise(|| ...)` to pick one branch from a reactive key:

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let tab = signal("code");
    column()
        .children([
            NodeBuilder::from(dot(Tone::Success)),
            text("Ready").into(),
        ])
        .child(
            switch(tab)
                .case("code", || text("Code view"))
                .case("diff", || text("Diff view"))
                .otherwise(|| text("Other")),
        )
}
```

### Fuzzy matching

`fuzzy_score(query, text)` returns `Option<FuzzyMatch>`. Call `.score()` (lower is better; `0` is an exact match) to rank hits and `.positions()` (`&[usize]`) for matched character indexes:

```rust
use sigil::prelude::*;
use sigil::widgets::fuzzy_score;

if let Some(hit) = fuzzy_score("cmd", "command_menu") {
    assert!(!hit.positions().is_empty());
    let _rank: usize = hit.score();
}
```

## Animation and tone steps

`Transition` configures CSS-style property transitions (`.duration(...)`, `.ease(Easing::CubicOut)`, `Easing::QuadOut`, etc.). `Widget::drive(Property::Opacity, signal)` binds a `Signal<f32>` directly to an animatable `Property`. `Tone::step(n)` (1–12) selects a step from a tone's color scale as a `Token`:

```rust,no_run
use sigil::animation::Property;
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let alpha = signal(1.0_f32);
    container()
        .background(Tone::Primary.step(3))
        .transition(
            Transition::opacity()
                .duration(Duration::from_millis(150))
                .ease(Easing::CubicOut),
        )
        .drive(Property::Opacity, alpha)
        .child(text("Fading panel"))
}
```

## Streaming from a backend

Signals belong to the UI thread. A backend task writes through a `SignalWriter`. The host runs each write on the UI thread at the start of the next turn. `SignalWriter::update` appends in place, so a long transcript is not copied on each chunk. Read it with `Signal::with` to avoid a clone.

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let transcript = signal(String::new());
    let writer = transcript.writer();
    let _done = spawn(async move {
        for word in ["streamed ", "from ", "a ", "task"] {
            sigil::tasks::sleep(Duration::from_millis(200)).await;
            writer.update(|text| text.push_str(word))?;
        }
        Ok::<(), Error>(())
    });
    text(move || transcript.with(|text| format!("{} bytes: {text}", text.len())))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("stream").terminal(), tree)
}
# }
# fn main() {}
```

The future runs on the app's tokio runtime. Wait inside it with `sigil::tasks::sleep`, not `std::thread::sleep`, so a worker thread stays free. `update` returns an `Error` with kind `ErrorKind::Closed` after the app exits. The `?` ends the task on that error.

For one result, `spawn` runs a future and returns a `Signal<Status<T, E>>` that starts as `Status::Pending`:

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let answer = spawn(async { Ok::<_, String>(6 * 7) });
    text(memo(move || match answer.get() {
        Status::Idle | Status::Pending => "working".to_string(),
        Status::Ok(value) => format!("answer: {value}"),
        Status::Err(err) => format!("error: {err}"),
        Status::Panicked(msg) => format!("panic: {msg}"),
    }))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("spawn"), tree)
}
```

### Load on demand

For work the user starts, such as a Load button, `task` builds a new future on each `run()`. The status starts `Status::Idle`. `run()` sets `Status::Pending` and aborts the run it replaces; a result from a replaced run is dropped, so a slow first load cannot overwrite a newer one.

```rust,ignore
{{#include ../../crates/sigil/examples/story-06-async.rs}}
```

### Stream a measured transcript

A chat transcript has rows whose height depends on the width: wrapped markdown, tool cards, images. `.measured()` sizes each row from its layout. `.follow_end(true)` keeps the last row in view while a reply streams in.

App state that is not `Send` stays on the UI thread. A worker sends messages on a tokio channel. `listen` runs the handler on the UI thread for each message. A send wakes the event loop, so the app does not poll.

```rust,no_run
use sigil::tasks::listen;
use sigil::prelude::*;
use std::rc::Rc;

enum Event {
    Start,
    Chunk(String),
}

fn tree() -> impl IntoNode {
    let messages: Signal<Rc<Vec<String>>> = signal(Rc::new(Vec::new()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    listen(rx, move |event| {
        messages.update(|list| match event {
            Event::Start => Rc::make_mut(list).push(String::new()),
            Event::Chunk(text) => {
                if let Some(last) = Rc::make_mut(list).last_mut() {
                    last.push_str(&text);
                }
            }
        });
    }, || {});
    std::thread::spawn(move || {
        drop(tx.send(Event::Start));
        for word in ["streamed ", "reply ", "text"] {
            drop(tx.send(Event::Chunk(word.into())));
        }
    });
    virtual_list(messages, |_, text: &String| markdown(signal(text.clone())))
        .measured()
        .estimate(rows(2.0))
        .fill()
        .follow_end(true)
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("transcript"), tree)
}
```

Page the transcript from outside the list. `.max_scroll` receives the largest scroll offset. Write `scroll_y` clamped to that value. `.follow` stops at the user's position after a scroll up, and follows again when the view is back at the bottom.

```rust,no_run
use sigil::prelude::*;
use std::rc::Rc;

fn tree() -> impl IntoNode {
    let messages: Signal<Rc<Vec<String>>> = signal(Rc::new(Vec::new()));
    let tail = memo(move || messages.get().len().saturating_sub(1));
    let (scroll, max) = (signal(0.0_f32), signal(0.0_f32));
    let page = 400.0;
    let list = virtual_list(messages, |_, text: &String| markdown(signal(text.clone())))
        .measured()
        .fill()
        .scroll_y(scroll)
        .max_scroll(max)
        .follow_row(tail);
    let root = container()
        .key(NamedKey::PageUp, move |_| scroll.set((scroll.peek() - page).clamp(0.0, max.peek())))
        .key(NamedKey::PageDown, move |_| scroll.set((scroll.peek() + page).clamp(0.0, max.peek())))
        .key(NamedKey::End, move |_| scroll.set(max.peek()));
    root.child(list)
}
```

`listen(rx, f, close)` runs `close` after the last message, when the worker drops its sender. `rx` may be a bounded `tokio::sync::mpsc::Receiver`.

### Consume a trigger inside an effect

An effect that reads a signal and writes the same signal inside its run is a reactive cycle. `defer(f)` runs `f` after the effect returns. The write then runs the effect again, and that run sees the cleared value. `untrack` does not help here: it stops the subscription of a read, not the notification of a write.

```rust,no_run
use sigil::reactive::defer;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let request: Signal<Option<String>> = signal(None);
    let log = signal(String::new());
    effect(move || {
        if let Some(line) = request.get() {
            log.update(|text| text.push_str(&line));
            defer(move || request.set(None));
        }
    });
    request.set(Some("first".into()));
    text(log)
}
```

## Timers

`interval(period, f)` runs `f` on the UI thread once per period. The interval stops when its owner is disposed, or when you call `id.cancel()`. It needs no thread.

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let left = signal(25 * 60_u32);
    let running = signal(true);
    let _tick: Timer = interval(Duration::from_secs(1), move || {
        if running.get() && left.get() > 0 {
            left.update(|s| *s -= 1);
        }
    });
    container()
        .child(text(memo(move || format!("{:02}:{:02}", left.get() / 60, left.get() % 60))))
        .child(button("Pause").activate(move || running.update(|r| *r = !*r)))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("timer").terminal(), tree)
}
# }
# fn main() {}
```

`spinner()` is a text node that uses the same interval registry.

## Host hooks

These are methods of `sigil::terminal::Terminal::current()`, and they work in the terminal host. Outside the terminal host, `suspend` runs its function, and the other methods do nothing.

| Call | Effect |
| --- | --- |
| `term.suspend(f)` | Restore the terminal, run `f`, then take the screen back. |
| `term.title(title)` | Set the terminal window title. |
| `term.bell(note)` | Ring the bell, and send a desktop notification if `note` is `Some`. |
| `term.is_focused()` | `true` if the terminal window has focus. |
| `sigil::exit()` | Exit after this frame, on any host. |

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;

fn edit() {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let term = sigil::terminal::Terminal::current();
    let status = term.suspend(|| std::process::Command::new(editor).status());
    let note = if matches!(status, Ok(Ok(code)) if code.success()) { "saved" } else { "editor failed" };
    drop(term.title(note));
    if !term.is_focused() {
        drop(term.bell(note));
    }
}

fn tree() -> impl IntoNode {
    container()
        .child(button("Edit").activate(edit))
        .child(button("Quit").activate(sigil::exit))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("hooks").terminal().exit_on_ctrl_c(false), tree)
}
# }
# fn main() {}
```

Ctrl+C always reaches the tree first. With `exit_on_ctrl_c(false)` an unconsumed Ctrl+C no longer exits, and the app exits through `exit`.

## Screen readers

`sigil::run` connects the window host to the platform screen reader. An embed host does this itself:

1. Call `Host::accessibility_tree()` after each layout. Send the `TreeUpdate` to your `accesskit` adapter.
2. When the adapter reports an `accesskit::ActionRequest`, convert its `target_node` with `NodeId::from_accesskit`.
3. Call `Host::perform(target, &request)`.

`Host::perform` handles `Focus`, `Click`, `Increment`, `Decrement`, `SetValue`, `SetTextSelection`, `Expand`, `Collapse`, and `ScrollIntoView`. Add `accesskit` 0.25 to your own dependencies to name its types.

```rust,no_run
use sigil::Host;
use sigil::app::HostMode;
use sigil::node::NodeId;
use sigil::prelude::*;
use sigil::reactive::Owner;

fn main() {
    Owner::scope("a11y", || {
        let mut host = Host::new(HostMode::Pixel);
        host.mount(|| button("Save").activate(|| println!("saved")));
        if host.layout((320.0, 200.0)).is_err() {
            return;
        }
        let update: accesskit::TreeUpdate = host.accessibility_tree();
        let Some(target) = update.nodes.first().map(|(id, _)| *id) else { return };
        let request = accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        };
        host.perform(NodeId::from_accesskit(request.target_node), &request);
    });
}
```

Give each control a name that a screen reader can say. Widgets take the name from their label. For a bare container, set `.role(Role::...)` and `.label("...")`.

## Tasks in an embed

`sigil::run` owns the task runtime. An embed host creates one on its UI thread with `AsyncRuntime::new()`, installs it with `install_current()`, and calls `pump()` before each `Host::frame`. `pump` applies the results that finished, so `task`, `spawn`, and `Signal::writer` work as they do under `sigil::run`. `waker` wakes the host's event loop when a result arrives.

```rust,no_run
use sigil::Host;
use sigil::app::{FrameHooks, HostMode};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::tasks::AsyncRuntime;

fn main() -> sigil::AppResult<()> {
    Owner::scope("embed", || {
        let mut tasks = AsyncRuntime::new()?;
        tasks.install_current();
        let mut host = Host::new(HostMode::Pixel);
        host.mount(|| {
            let load = task(|| async { Ok::<_, String>("ready".to_owned()) });
            load.run();
            text(move || match load.status() {
                Status::Ok(value) => value,
                _ => "loading".to_owned(),
            })
        });
        tasks.pump();
        drop(host.frame((320.0, 200.0), 1.0 / 60.0, FrameHooks::default()));
        Ok(())
    })
}
```
