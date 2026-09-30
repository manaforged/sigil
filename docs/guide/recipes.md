# Recipes

Each recipe is a complete program or tree function for one job. Every
code block on this page compiles as a doctest of the `sigil` crate. The
[API map](../api.md) lists the signature of each item used here.

## Keys

A key event goes to the focused node first. Handlers run in two passes
along the path from the root to the focused node:

1. Capture handlers (`key_capture`, `key_capture_any`) run from the
   root down to the focused node.
2. Bubble handlers (`key`, `key_any`) run from the focused node up to
   the root.

A handler calls `event.stop_propagation()` to end both passes.

A `Shortcut` is a key plus modifiers:

| Constructor | Matches |
| --- | --- |
| `Shortcut::plain(Key::Enter)` | Enter with no modifiers |
| `Shortcut::plain(LogicalKey::character("j"))` | The `j` key with no modifiers |
| `Shortcut::ctrl(LogicalKey::character("s"))` | Ctrl+S |
| `Shortcut::new(key, ModifiersState::CONTROL \| ModifiersState::SHIFT)` | Any modifier set |

`Key::Escape` converts into a `Shortcut` directly, so `.key(Key::Escape, f)`
also works.

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
        .key(LogicalKey::character("j"), move |_| count.update(|n| *n += 1))
        .key(LogicalKey::character("k"), move |_| count.update(|n| *n -= 1))
        .key(Key::Escape, |_| sigil::request_exit())
        .child(text(derived(move || format!("row {}", count.get()))))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("keys").terminal(), tree)
}
# }
# fn main() {}
```

`request_focus` gives the node keyboard focus. Without focus, a node
receives no key events through the focus path.

### Bind a key that fires from anywhere

`.shortcut(s, handler)` registers `s` as a global shortcut. The router
runs the handler for the key, wherever the focus is. One node can hold
many shortcuts. A plain character shortcut does not fire while a text
input has focus, so typing `q` into a field does not quit.

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let name = signal(String::new());
    container()
        .shortcut(LogicalKey::character("q"), |_| sigil::request_exit())
        .shortcut(Shortcut::ctrl(LogicalKey::character("n")), move |_| name.set(String::new()))
        .child(text_field(name))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("global keys").terminal(), tree)
}
# }
# fn main() {}
```

### See every key before the focused widget

`key_capture_any` on an ancestor sees each key before the focused
descendant. This example turns Tab inside a text editor into two
spaces.

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let body = signal(String::new());
    container().child(text_edit(body)).key_capture_any(move |event: &mut KeyEvent| {
        if event.key == LogicalKey::named(Key::Tab) {
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

## Theme switching

`Theme::light()`, `Theme::dark()`, and `Theme::console_dark()` build
the default themes. `set_theme` replaces the active theme at the next
frame, on the window and on the terminal host.

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let dark = signal(false);
    container().child(toggle(dark)).child(button("Apply").activate(move || {
        set_theme(if dark.get() { Theme::dark() } else { Theme::light() });
    }))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("themes"), tree)
}
```

A theme file is TOML. `base` selects `light`, `dark`, or
`console_dark`. The `[tokens]` table sets token colours as `#rrggbb`
or `#rrggbbaa`. `parse_theme` returns `None` if the document is not
valid.

```rust,no_run
use sigil::app::parse_theme;
use sigil::prelude::*;

const SOURCE: &str = r##"
base = "dark"

[tokens]
accent = "#3c59e7"
surface = "#101014"
"##;

fn tree() -> impl IntoNode {
    if let Some(theme) = parse_theme(SOURCE) {
        set_theme(theme);
    }
    text("Custom theme")
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("theme file"), tree)
}
```

To reload a theme file while the app runs, call `watch_theme`. The
watch stops when the `ThemeWatcher` is dropped, so keep it for the
life of the tree. `AppConfig::theme_file(path)` loads and watches a
file without code in the tree.

```rust,no_run
use sigil::reactive::on_cleanup;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    match watch_theme("theme.toml") {
        Ok(watcher) => on_cleanup(move || drop(watcher)),
        Err(err) => eprintln!("theme watch failed: {err}"),
    }
    text("Edit theme.toml")
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("watch").theme_file("base.toml"), tree)
}
```

`Token` names each theme colour. The `Paint` methods `bg`,
`text_color`, and `border_color` take a token, so a misspelled token
does not compile.

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    container()
        .bg(Token::SurfaceSubtle)
        .border_w(1)
        .border_color(Token::Border)
        .child(text("Panel").text_color(Token::OnSurfaceMuted))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("tokens"), tree)
}
```

## Layout reference

`LayoutStyle` describes the box of one node. Start from a constructor,
then chain setters:

| Constructor | Box |
| --- | --- |
| `LayoutStyle::flex_column()` | Children in a column |
| `LayoutStyle::flex_row()` | Children in a row |
| `LayoutStyle::grid(tracks)` | Children in grid columns |
| `LayoutStyle::block()` | Block flow |

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

A plain number is logical pixels. `rows(n)`, `cols(n)`, and `em(n)`
are text-relative. In the terminal host, `rows(1.0)` is one cell high
and `cols(1.0)` is one cell wide. In a window, `rows(1.0)` is one line
height and `cols(1.0)` is one character advance. Use text-relative
units in a tree that runs on both hosts. See [Units](units.md).

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::layout::TrackSize;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let header = container()
        .layout(LayoutStyle::flex_row().height(rows(1.0)).gap(cols(1.0)).fill_width())
        .child(text("Title"))
        .child(spacer())
        .child(text("v0.1"));
    let grid = container()
        .layout(LayoutStyle::grid(vec![TrackSize::Px(160.0), TrackSize::Fr(1.0)]).gap(rows(1.0)))
        .child(text("Name"))
        .child(text("Sigil"));
    let body = container()
        .layout(LayoutStyle::flex_column().grow(1.0).padding(cols(2.0)).max_width(em(40.0)))
        .child(grid);
    container().layout(LayoutStyle::flex_column().fill()).child(header).child(body)
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("layout").terminal(), tree)
}
# }
# fn main() {}
```

`viewport_size()` returns the viewport as a signal, in logical px on
every host. On the terminal one cell is `Host::CELL` px, so divide by
it to get columns and rows. Read it in a `derived` or a
`children_reactive` closure to adapt the tree to the size.

```rust,no_run
use sigil::app::is_grid;
use sigil::app::viewport_size;
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let size = viewport_size();
    let wide = derived(move || size.get().width > if is_grid() { 100.0 } else { 900.0 });
    text(derived(move || if wide.get() { "two columns".to_string() } else { "one column".to_string() }))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("viewport"), tree)
}
```

## Streaming from a backend

Signals belong to the UI thread. A backend task writes through a
`SignalWriter`. The host runs each write on the UI thread at the start
of the next turn. `SignalWriter::update` appends in place, so a long
transcript is not copied on each chunk. Read it with `Signal::with` to
avoid a clone.

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let transcript = signal(String::new());
    let writer = signal_writer(transcript);
    let _done = spawn(async move {
        for word in ["streamed ", "from ", "a ", "task"] {
            sigil::tasks::sleep(Duration::from_millis(200)).await;
            writer.update(|text| text.push_str(word))?;
        }
        Ok::<(), ScheduleError>(())
    });
    text(move || transcript.with(|text| format!("{} bytes: {text}", text.len())))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("stream").terminal(), tree)
}
# }
# fn main() {}
```

The future runs on the app's tokio runtime. Wait inside it with
`sigil::tasks::sleep`, not `std::thread::sleep`, so a worker thread
stays free. `update` returns `Err(ScheduleError::Closed)` after the app
exits. The `?` ends the task on that error.

For one result, `spawn` runs a future and returns a
`Signal<Status<T, E>>` that starts as `Status::Pending`:

```rust,no_run
use sigil::prelude::*;

fn tree() -> impl IntoNode {
    let answer = spawn(async { Ok::<_, String>(6 * 7) });
    text(derived(move || match answer.get() {
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

For work the user starts, such as a Load button, `task` builds a new
future on each `run()`. The status starts `Status::Idle`. `run()` sets
`Status::Pending` and aborts the run it replaces; a result from a
replaced run is dropped, so a slow first load cannot overwrite a newer
one.

```rust,ignore
{{#include ../../crates/sigil/examples/story-07-async.rs}}
```

### Stream a measured transcript

A chat transcript has rows whose height depends on the width: wrapped
markdown, tool cards, images. `.measured()` sizes each row from its
layout. `.auto_pin_to_bottom(true)` keeps the last row in view while a
reply streams in.

App state that is not `Send` stays on the UI thread. A worker sends
messages on a tokio channel. `on_message` runs the handler on the UI
thread for each message. A send wakes the event loop, so the app does
not poll.

```rust,no_run
use sigil::tasks::on_message;
use sigil::prelude::*;
use std::rc::Rc;

enum Event {
    Start,
    Chunk(String),
}

fn tree() -> impl IntoNode {
    let messages: Signal<Rc<Vec<String>>> = signal(Rc::new(Vec::new()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    on_message(rx, move |event| {
        messages.update(|list| match event {
            Event::Start => Rc::make_mut(list).push(String::new()),
            Event::Chunk(text) => {
                if let Some(last) = Rc::make_mut(list).last_mut() {
                    last.push_str(&text);
                }
            }
        });
    });
    std::thread::spawn(move || {
        drop(tx.send(Event::Start));
        for word in ["streamed ", "reply ", "text"] {
            drop(tx.send(Event::Chunk(word.into())));
        }
    });
    virtual_list(messages, |_, text: &String| markdown(signal(text.clone())))
        .measured()
        .estimate(rows(2.0))
        .viewport_fill()
        .auto_pin_to_bottom(true)
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("transcript"), tree)
}
```

Page the transcript from outside the list. `.max_scroll_bind` receives
the largest scroll offset. Write `scroll_bind` clamped to that value.
`.follow` stops at the user's position after a scroll up, and follows
again when the view is back at the bottom.

```rust,no_run
use sigil::prelude::*;
use std::rc::Rc;

fn tree() -> impl IntoNode {
    let messages: Signal<Rc<Vec<String>>> = signal(Rc::new(Vec::new()));
    let tail = derived(move || messages.get().len().saturating_sub(1));
    let (scroll, max) = (signal(0.0_f32), signal(0.0_f32));
    let page = 400.0;
    let list = virtual_list(messages, |_, text: &String| markdown(signal(text.clone())))
        .measured()
        .viewport_fill()
        .scroll_bind(scroll)
        .max_scroll_bind(max)
        .follow(tail);
    let root = container()
        .key(Key::PageUp, move |_| scroll.set((scroll.peek() - page).clamp(0.0, max.peek())))
        .key(Key::PageDown, move |_| scroll.set((scroll.peek() + page).clamp(0.0, max.peek())))
        .key(Key::End, move |_| scroll.set(max.peek()));
    root.child(list)
}
```

`on_message_or_close(rx, f, on_close)` also runs `on_close` after the
last message, when the worker drops its sender. `rx` may be a bounded
`tokio::sync::mpsc::Receiver`.

### Consume a trigger inside an effect

An effect that reads a signal and writes the same signal inside its
run is a reactive cycle. `defer(f)` runs `f` after the effect returns.
The write then runs the effect again, and that run sees the cleared
value. `untrack` does not help here: it stops the subscription of a
read, not the notification of a write.

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

`interval(period, f)` runs `f` on the UI thread once per period. The
interval stops when its owner is disposed, or when you call
`sigil::reactive::cancel_interval(id)`. It needs no thread.

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;
use std::time::Duration;

fn tree() -> impl IntoNode {
    let left = signal(25 * 60_u32);
    let running = signal(true);
    let _tick: IntervalId = interval(Duration::from_secs(1), move || {
        if running.get() && left.get() > 0 {
            left.update(|s| *s -= 1);
        }
    });
    container()
        .child(text(derived(move || format!("{:02}:{:02}", left.get() / 60, left.get() % 60))))
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

These functions work in the terminal host. Outside the terminal host,
`suspend` runs its function, and the other functions do nothing.

| Function | Effect |
| --- | --- |
| `sigil::terminal::suspend(f)` | Restore the terminal, run `f`, then take the screen back. |
| `sigil::terminal::set_title(title)` | Set the terminal window title. |
| `sigil::terminal::bell(note)` | Ring the bell, and send a desktop notification if `note` is `Some`. |
| `sigil::terminal::focused()` | `true` if the terminal window has focus. |
| `sigil::request_exit()` | Exit after this frame, on any host. |

```rust,no_run
# #[cfg(feature = "terminal")]
# mod terminal_only {
use sigil::prelude::*;

fn edit() {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = sigil::terminal::suspend(|| std::process::Command::new(editor).status());
    let note = if matches!(status, Ok(Ok(code)) if code.success()) { "saved" } else { "editor failed" };
    drop(sigil::terminal::set_title(note));
    if !sigil::terminal::focused() {
        drop(sigil::terminal::bell(Some(note)));
    }
}

fn tree() -> impl IntoNode {
    container()
        .child(button("Edit").activate(edit))
        .child(button("Quit").activate(sigil::request_exit))
}

fn main() -> AppResult<()> {
    sigil::run(AppConfig::new("hooks").terminal().quit_on_ctrl_c(false), tree)
}
# }
# fn main() {}
```

With `quit_on_ctrl_c(false)`, Ctrl+C reaches the tree as a key event,
and the app exits through `request_exit`.

## Screen readers

`sigil::run` connects the window host to the platform screen reader.
An embed host does this itself:

1. Call `Host::accessibility_tree(&router)` after each layout. Send the
   `TreeUpdate` to your `accesskit` adapter.
2. When the adapter reports an `accesskit::ActionRequest`, convert its
   `target_node` with `NodeId::from_accesskit`.
3. Call `Host::perform(&mut router, target, &request)`.

`Host::perform` handles `Focus`, `Click`, `Increment`, `Decrement`,
`SetValue`, `SetTextSelection`, `Expand`, `Collapse`, and
`ScrollIntoView`. Add `accesskit` 0.25 to your own dependencies to
name its types.

```rust,no_run
use sigil::kernel::NodeId;
use sigil::prelude::*;
use sigil::events::EventRouter;
use sigil::Host;
use sigil::reactive::Runtime;

fn main() {
    Runtime::scope("a11y", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(|| button("Save").activate(|| println!("saved")));
        if host.layout((320.0, 200.0)).is_err() {
            return;
        }
        let update: sigil::app::AccessibilityTree = host.accessibility_tree(&router);
        let Some(target) = update.nodes.first().map(|(id, _)| *id) else { return };
        let request = accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            data: None,
        };
        host.perform(&mut router, NodeId::from_accesskit(request.target_node), &request);
    });
}
```

Give each control a name that a screen reader can say. Widgets take
the name from their label. For a bare container, set
`.role(Role::...)` and `.label("...")`.

## Tasks in an embed

`sigil::run` owns the task runtime. An embed host creates one on its UI
thread with `AsyncRuntime::new()`, installs it with `install_current()`,
and calls `pump()` before each `Host::frame`. `pump` applies the
results that finished, so `task`, `spawn`, and `signal_writer` work as
they do under `sigil::run`. `set_waker` wakes the host's event loop
when a result arrives.

```rust,no_run
use sigil::app::FrameHooks;
use sigil::prelude::*;
use sigil::events::EventRouter;
use sigil::Host;
use sigil::reactive::Runtime;
use sigil::tasks::AsyncRuntime;

fn main() -> std::io::Result<()> {
    Runtime::scope("embed", || {
        let mut tasks = AsyncRuntime::new()?;
        tasks.install_current();
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(|| {
            let load = task(|| async { Ok::<_, String>("ready".to_owned()) });
            load.run();
            text(move || match load.status() {
                Status::Ok(value) => value,
                _ => "loading".to_owned(),
            })
        });
        tasks.pump();
        drop(host.frame(&mut router, (320.0, 200.0), 1.0 / 60.0, FrameHooks::default()));
        Ok(())
    })
}
```
