A screen read from `.sigil` text. In a debug build it reloads each time the file is saved.

`sigil::view!("src/screen.sigil")` embeds the file and watches it in debug builds. Bind state by name with `signal`, `flag`, and `action`, and put a tree built in Rust in a `slot`. A save with a mistake keeps the previous screen and shows the error; `check` returns the same error for a test.

```rust
use sigil::View;
use sigil::prelude::*;

fn settings() -> impl IntoNode {
    let name = signal(String::new());
    View::new("column gap=lg {\n  text_field value=$name placeholder=\"Your name\"\n}\n")
        .signal("name", name)
}
```

See also: [Hot reload](https://github.com/manaforged/sigil/blob/main/docs/guide/hot-reload.md).
