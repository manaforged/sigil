# Testing

`sigil_kittest::Harness` drives a widget tree the way a user does: it
finds controls through the accessibility tree, clicks and types through
the normal event router, and runs frames until the UI is idle. The same
test runs on the terminal host or the window host.

Add the harness as a dev-dependency:

```toml
[dev-dependencies]
sigil-kittest = { git = "https://github.com/manaforged/sigil" }
```

## A first test

```rust
use sigil::prelude::*;
use sigil_kittest::Harness;

fn counter(cx: &mut Cx) -> NodeId {
    let count = signal(0_i32);
    let label = derived(move || format!("Count: {}", count.get()));
    cx.add(
        container()
            .child(text(label))
            .child(button("Increment").activate(move || count.set(count.peek() + 1))),
    )
}

let mut ui = Harness::new(counter).size(40, 6);
let increment = ui.get_by_role(Role::Button, "Increment");
ui.click(&increment);
assert!(ui.text().contains("Count: 1"));
```

`size(cols, rows)` uses the terminal cell host; the default is 80×24.
`pixels(width, height)` uses the window host instead. Queries and
actions behave the same on both.

## Find controls

Queries read the accessibility tree, so a test finds a control by what a
user or a screen reader sees:

| Call | Matches |
| --- | --- |
| `get_by_role(Role::Button, "Save")` | role and accessible name |
| `get_by_text("Hello")` | a node whose name or value is the text |
| `get(&Query::role(Role::CheckBox))` | any `Query` |
| `find_all(&query)`, `query(&query)` | every match, or the first |
| `focused()` | the focused node |

`get*` fails the test and prints the accessibility tree unless exactly
one node matches. A field inside `field("Name", ..)` takes its name from
the label.

A `Node` is a snapshot. Read `value()`, `is_checked()`, `is_focused()`
and the other accessors on it, and query again, or call
`ui.refresh(&node)`, after an action.

```rust
# use sigil::prelude::*;
# use sigil_kittest::Harness;
fn form(cx: &mut Cx) -> NodeId {
    let name = signal(String::new());
    let agree = signal(false);
    cx.add(
        container()
            .layout(LayoutStyle::flex_column())
            .child(field("Name", text_field(name)))
            .child(checkbox(agree).label("Agree")),
    )
}

let mut ui = Harness::new(form).size(40, 12);
let name = ui.get_by_role(Role::TextInput, "Name");
ui.click(&name).type_text("Ada");
assert_eq!(ui.get_by_role(Role::TextInput, "Name").value(), Some("Ada"));

let agree = ui.get_by_role(Role::CheckBox, "Agree");
ui.click(&agree);
assert_eq!(ui.refresh(&agree).and_then(|node| node.is_checked()), Some(true));
```

## Act

| Call | Sends |
| --- | --- |
| `click(&target)` | move, press, release at the target centre |
| `hover(&target)` | a pointer move |
| `scroll(&target, lines)` | a wheel event over the target |
| `drag(&from, &to)` | press, four moves with the button held, release |
| `press(Key::Enter)`, `press_with(key, modifiers)` | key down and up |
| `type_text("abc")` | one key per character |
| `focus_next()`, `focus_previous()` | Tab, Shift+Tab |
| `send(events)` | any `ShellEvent`s, built with `sigil_kittest::input` |

A target is a `Node` or a `Point` in layout points. Every action runs
frames until no signal write and no animation is pending. A tree that
never settles fails the test after `SETTLE_SAFETY_CAP` frames.

## Time

The harness owns the interval clock. `advance(duration)` moves it
forward and runs a frame per 1/60 s, so intervals fire and animations
step without real waiting:

```rust
# use std::time::Duration;
# use sigil::prelude::*;
# use sigil_kittest::Harness;
let ticks = |cx: &mut Cx| {
    let n = signal(0_u32);
    interval(Duration::from_secs(1), move || n.set(n.peek() + 1));
    cx.add(text(derived(move || format!("ticks {}", n.get()))))
};
let mut ui = Harness::new(ticks).size(20, 2);
ui.advance(Duration::from_secs(3));
assert!(ui.text().contains("ticks 3"));
```

## Snapshots

`snapshot("name")` compares the harness output with a committed golden
under `tests/goldens/` of the crate that runs the test:

- A cell harness writes the grid text to `<name>.txt`.
- A pixel harness renders offscreen and compares `<name>.png` within
  `SNAPSHOT_MAX_DELTA_PCT`. `snapshot_with(name, pct)` sets the budget.

`SIGIL_REGEN_GOLDENS=1` rewrites the goldens. A pixel snapshot without a
GPU adapter fails unless `SIGIL_ALLOW_NO_GPU=1` is set, which skips it.

```rust,no_run
# use sigil::prelude::*;
# use sigil_kittest::Harness;
let mut ui = Harness::new(|cx: &mut Cx| cx.add(button("Save"))).pixels(200, 60);
ui.snapshot("save_button");
```

## Lower level

`ui.host()` returns the `Host` and `EventRouter` for anything the
harness does not cover. `ui.accessibility_snapshot()` returns the text
dump that a failed `get` prints. [Headless driving](automation.md)
describes the host calls the harness is built on.
