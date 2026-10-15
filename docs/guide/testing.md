# Testing

`sigil_kittest::Harness` drives a widget tree the way a user does: it finds controls through the accessibility tree, clicks and types through the normal event router, and runs frames until the UI is idle. The same test runs on the terminal host or the window host.

Add the harness as a dev-dependency:

```toml
[dev-dependencies]
sigil-kittest = { git = "https://github.com/manaforged/sigil" }
```

## A first test

```rust
use sigil::prelude::*;
use sigil_kittest::Harness;

fn counter() -> impl IntoNode {
    let count = signal(0);
    row()
        .child(text(move || format!("Count: {}", count.get())))
        .child(button("+").activate(move || count.update(|n| *n += 1)))
}

Harness::new(counter).click("+").expect_text("Count: 1");
```

`click("+")` finds the control a user sees as "+". A label names the text on a button or tab, a checkbox label, or a field's label. When the label matches a control and a plain text, the control wins; a label on two controls fails the test. `expect_text` waits until a node shows the text, up to `EXPECT_TIMEOUT` (5 s), then fails with the accessibility tree. Every action runs frames until the UI settles, so a test never calls `settle` itself.

`size(cols, rows)` uses the terminal cell host; the default is 80×24. `pixels(width, height)` uses the window host instead. Queries and actions behave the same on both.

## Find controls

Queries read the accessibility tree, so a test finds a control by what a user or a screen reader sees:

| Call | Matches |
| --- | --- |
| `expect_text("Saved")` | waits for a node whose name or value is the text |
| `get_by_role(Role::Button, "Save")` | role and accessible name |
| `get_by_text("Hello")` | a node whose name or value is the text |
| `get(&Query::role(Role::CheckBox))` | any `Query` |
| `find_all(&query)`, `query(&query)` | every match, or the first |
| `focused()` | the focused node |

`get*` fails the test and prints the accessibility tree unless exactly one node matches. A field inside `field("Name", ..)` takes its name from the label.

A `Node` is a snapshot. Read `value()`, `is_checked()`, `is_focused()` and the other accessors on it, and query again, or call `ui.refresh(&node)`, after an action.

```rust
# use sigil::prelude::*;
# use sigil_kittest::Harness;
fn form() -> impl IntoNode {
    let name = signal(String::new());
    let agree = signal(false);
    container()
        .layout(LayoutStyle::flex_column())
        .child(field("Name", text_field(name)))
        .child(checkbox(agree).label("Agree"))
}

let mut ui = Harness::new(form).size(40, 12);
ui.type_into("Name", "Ada").click("Agree");
assert_eq!(ui.get_by_role(Role::TextInput, "Name").value(), Some("Ada"));
assert_eq!(ui.get_by_role(Role::CheckBox, "Agree").is_checked(), Some(true));
```

## Act

| Call | Sends |
| --- | --- |
| `click("Save")`, `click(&node)` | move, press, release at the target centre |
| `hover(&target)` | a pointer move |
| `scroll(&target, lines)` | a wheel event over the target |
| `drag(&from, &to)` | press, four moves with the button held, release |
| `press(Key::Enter)`, `press_with(key, modifiers)` | key down and up |
| `type_text("abc")` | one key per character |
| `type_into("Name", "Ada")` | a click on the control labelled Name, then the text |
| `focus_next()`, `focus_previous()` | Tab, Shift+Tab |
| `send(events)` | any `ShellEvent`s, built with `sigil_kittest::input` |

A target is a label, a `Node`, or a `Point` in layout points. A label waits up to `EXPECT_TIMEOUT` for its control to appear. Every action runs frames until no signal write and no animation is pending. A tree that never settles fails the test after `SETTLE_SAFETY_CAP` frames.

The harness installs a task runtime, so `task`, `spawn`, and `signal_writer` work in a test. Each frame applies the results that finished; `expect_text` waits for them:

```rust,ignore
{{#include ../../crates/sigil/examples/story-06-async.rs}}
```

```rust,ignore
Harness::new(loader)
    .click("Load")
    .expect_text("Loading…")
    .expect_text("42 items");
```

## Time

The harness owns the interval clock. `advance(duration)` moves it forward and runs a frame per 1/60 s, so intervals fire and animations step without real waiting:

```rust
# use std::time::Duration;
# use sigil::prelude::*;
# use sigil_kittest::Harness;
let ticks = || {
    let n = signal(0_u32);
    interval(Duration::from_secs(1), move || n.set(n.peek() + 1));
    text(derived(move || format!("ticks {}", n.get())))
};
let mut ui = Harness::new(ticks).size(20, 2);
ui.advance(Duration::from_secs(3));
assert!(ui.text().contains("ticks 3"));
```

## Snapshots

`snapshot("name")` compares the harness output with a committed golden under `tests/goldens/` of the crate that runs the test:

- A cell harness writes the grid text to `<name>.txt`.
- A pixel harness renders offscreen and compares `<name>.png` within `SNAPSHOT_MAX_DELTA_PCT`. `snapshot_with(name, pct)` sets the budget.

`SIGIL_REGEN_GOLDENS=1` rewrites the goldens. A pixel snapshot without a GPU adapter fails unless `SIGIL_ALLOW_NO_GPU=1` is set, which skips it.

```rust,no_run
# use sigil::prelude::*;
# use sigil_kittest::Harness;
let mut ui = Harness::new(|| button("Save")).pixels(200, 60);
ui.snapshot("save_button");
```

## Lower level

`ui.host()` returns the `Host` and `EventRouter` for anything the harness does not cover. `ui.accessibility_snapshot()` returns the text dump that a failed `get` prints. [Headless driving](automation.md) describes the host calls the harness is built on.

## Real terminals

The cell grid that tests read is what the terminal host sends, not what a terminal draws. Terminals differ in how they draw block and box glyphs from the font, so check a terminal screen in the terminals your users run. On macOS, `scripts/term-shot.sh` runs a command in a new Terminal.app, Ghostty, or iTerm window at a set size, waits, saves a PNG of that window, and closes it:

```sh
scripts/term-shot.sh --app Terminal --size 120x36 --font SFMono-Regular --font-size 11 \
  --env SIGIL_FILLS=whole --keys ctrl+o --wait 4 --out settings.png -- cargo run --example hello-counter
```

`--font` and `--font-size` set the window's font in Terminal.app through a temporary profile that is deleted afterwards, and in Ghostty through its command line; iTerm uses its default profile. `--env` exports a variable inside the new window, which does not inherit the calling shell's environment. `--keys` sends key chords to the captured window just before the capture and needs Peekaboo.

The shell that runs it needs Screen Recording permission for `screencapture`; where only Peekaboo holds that permission, the script captures through Peekaboo's bridge instead. A first launch of Ghostty or iTerm can stop on a macOS prompt that a person answers once.
