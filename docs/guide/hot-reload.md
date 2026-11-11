# Hot reload

There are two ways to change a running app. A view file changes the widget tree with no compiler and no extra tool. The `hot` feature patches Rust functions. Start with a view file when you iterate on shapes and layout.

## View files

A `.sigil` file describes a widget tree. `sigil::view!` reads a file relative to `Cargo.toml`, embeds it in the binary, and in a debug build watches the file. The screen changes within 200 ms of a save, with no rebuild.

```text
// settings.sigil
column gap=lg padding=xl {
  text "Settings" role=title
  field "Name" {
    text_field value=$name placeholder="Your name"
  }
  text "Saved" show=$saved
  row gap=sm {
    button "Save" primary activate=@save
  }
  slot rows
}
```

```rust,ignore
sigil::view!("src/settings.sigil")
    .signal("name", name)
    .flag("saved", saved)
    .action("save", move || saved.set(true))
    .slot("rows", move || rows(items))
```

Each line is one widget. A `{ }` block holds its children.

| Widget | Takes | Properties |
| --- | --- | --- |
| `column`, `row`, `card`, `scroll` | children | `gap` |
| `field` | `"label"` or `$name`, and one child | |
| `text` | `"words"` or `$name` | `role` |
| `button` | `"label"` or `$name` | `primary`, `outline`, `ghost`, `destructive`, `activate=@name`, `disabled=$name` |
| `text_field`, `text_edit` | | `value=$name` (required), `placeholder`, `disabled=$name`, `submit=@name` |
| `checkbox`, `toggle` | optional `"label"` or `$name` | `checked=$name` (required), `disabled=$name`, `change=@name` |
| `divider` | | |
| `spacer` | | |
| `slot` | a name | |

Every widget also takes `padding`, `layer`, `class="..."`, and `show=$name`.

- `gap` and `padding` take a step: `none`, `xxs`, `xs`, `sm`, `md`, `lg`, `xl`, `xxl`, `xxxl`. A number is an error.
- `role` takes `caption`, `body`, `label`, `subtitle`, `title`, `heading`, or `display`.
- `layer` takes `page`, `sunken`, `raised`, or `overlay`.
- `$name` binds a signal or a flag, and `@name` binds an action. A name with no binding is an error.
- `slot` holds a tree that you build in Rust, for a chart, a list, or any other widget that the file cannot name.

Signals live in Rust, so their values and the typed text stay when you save. If a save has a mistake, the previous screen stays and an error band names the line and the column. The next good save removes the band. `View::check` returns the same error, for a test:

```rust,ignore
assert!(sigil::view!("src/settings.sigil").signal("name", name).check().is_ok());
```

A release build uses the embedded text and does not watch the file. `View::new(text).watch(path)` gives the same behavior with a path that you choose. The view file works on the window host and the terminal host. `Harness` holds a view to the same design rules as any other tree. Run the example with `cargo run -p sigil-ui --example view-form --features desktop` and edit `examples/view-form.sigil`.

## Patch Rust functions

The `hot` feature of `sigil-ui` patches a running app when you save a source file. The window does not close and the app does not restart. Sigil uses [`subsecond`](https://crates.io/crates/subsecond), the hot-patch engine of Dioxus.

The feature is for development. Do not turn it on in a release build.

## Setup

Install the Dioxus command-line tool, version 0.7.10:

```bash
cargo install dioxus-cli@0.7.10 --locked
```

This installs the `dx` command.

## Run

Start the app under `dx` with the `hot` feature. In this repository:

```bash
dx serve --hot-patch -p sigil-ui --example hot-counter --features "hot terminal"
```

For your own binary, add the feature to your `sigil-ui` dependency, or pass `--features sigil-ui/hot`, and run `dx serve --hot-patch` in the binary crate.

Edit a function that builds the tree and save the file. `dx` compiles the change and sends the patch to the app. The next frame mounts the root again with the new code.

Without `dx`, the same binary runs normally. `sigil::run` connects to the devserver only when `dx` sets its environment variables.

## State

After a patch, Sigil calls the root closure again:

- A signal that you make outside the root closure, for example in `main`, keeps its value.
- A signal that you make inside the root closure starts again from its first value.
- Focus returns to the node with the same role and name path, if that node still exists.

Keep state that must survive a patch outside the root closure:

```rust,no_run
use sigil::prelude::*;
use sigil::reactive::Owner;

fn main() -> sigil::AppResult<()> {
    Owner::scope("app", || {
        let count = signal(0_i32);
        sigil::run(AppConfig::new("Counter"), move || counter(count))
    })
}

fn counter(count: Signal<i32>) -> NodeBuilder {
    let label = derived(move || format!("Count: {}", count.get()));
    container()
        .child(text(label))
        .child(button("Increment").activate(move || count.set(count.peek() + 1)))
}
```

Change the `"Count: {}"` text and save. The label changes and the count stays.

The root passed to `run` is `Fn` and `'static` in every build, so hot reload needs no change to it. A `move` closure that copies signals is both.

## Embed hosts

An embed host calls `Host::mount_hot(root)` in place of `Host::mount`. `Host::frame` mounts the root again after a patch.

## Limits

- Only the binary crate patches. A change in a library crate, Sigil included, needs a restart.
- A change to the layout of a type, such as a new struct field, needs a restart.
- `dx serve` captures the standard output of the app, so the terminal host cannot draw under it. Use the window host while you edit.
- `hot` does nothing on wasm.
- Platforms: macOS, Linux, and Windows.
