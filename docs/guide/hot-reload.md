# Hot reload

The `hot` feature of `sigil-ui` patches a running app when you save a
source file. The window does not close and the app does not restart.
Sigil uses [`subsecond`](https://crates.io/crates/subsecond), the
hot-patch engine of Dioxus.

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

For your own binary, add the feature to your `sigil-ui` dependency, or
pass `--features sigil-ui/hot`, and run `dx serve --hot-patch` in the
binary crate.

Edit a function that builds the tree and save the file. `dx` compiles
the change and sends the patch to the app. The next frame mounts the
root again with the new code.

Without `dx`, the same binary runs normally. `sigil::run` connects to
the devserver only when `dx` sets its environment variables.

## State

After a patch, Sigil calls the root closure again:

- A signal that you make outside the root closure, for example in
  `main`, keeps its value.
- A signal that you make inside the root closure starts again from its
  first value.
- Focus returns to the node with the same role and name path, if that
  node still exists.

Keep state that must survive a patch outside the root closure:

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    Runtime::scope("app", || {
        let count = signal(0_i32);
        sigil::run(AppConfig::new("Counter"), move |cx: &mut Cx| {
            cx.add(counter(count))
        })
    })
}

fn counter(count: Signal<i32>) -> NodeBuilder {
    let label = derived(move || format!("Count: {}", count.get()));
    container()
        .child(text(label))
        .child(button("Increment").activate(move || count.set(count.peek() + 1)))
}
```

Change the `"Count: {}"` text and save. The label changes and the count
stays.

With `hot` on, a closure root must be `Fn` and `'static`. A `move`
closure that copies signals is both. A node root such as
`sigil::root(node)` mounts once and does not patch.

## Embed hosts

An embed host calls `Host::mount_hot(root)` in place of `Host::mount`.
`Host::frame` mounts the root again after a patch.

## Limits

- Only the binary crate patches. A change in a library crate, Sigil
  included, needs a restart.
- A change to the layout of a type, such as a new struct field, needs a
  restart.
- `dx serve` captures the standard output of the app, so the terminal
  host cannot draw under it. Use the window host while you edit.
- `hot` does nothing on wasm.
- Platforms: macOS, Linux, and Windows.
