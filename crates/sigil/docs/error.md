The one error type of the crate. `kind()` says what failed.

Branch on [`ErrorKind`](crate::ErrorKind), never on the message. `source()` returns the cause. `Error::node` returns the missing node of an `ErrorKind::NodeMissing` error.

```rust,no_run
# #[cfg(any(feature = "desktop", feature = "terminal"))]
# mod app {
use sigil::prelude::*;
use sigil::ErrorKind;

fn main() {
    match sigil::run(AppConfig::new("App"), || text("Hello")) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::AlreadyRun => eprintln!("the app is already running"),
        Err(err) => eprintln!("{err}"),
    }
}
# }
# fn main() {}
```

See also: [Errors](https://github.com/manaforged/sigil/blob/main/docs/guide/errors.md).
