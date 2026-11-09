The settings that `sigil::run` starts an app with: title, window size, host, theme file, and fonts.

`AppConfig::new(title)` opens a window. `.terminal()` runs the same tree in the terminal, and `.inline(true)` keeps it below the shell prompt instead of on the alternate screen.

```rust,no_run
# #[cfg(any(feature = "desktop", feature = "terminal"))]
# mod app {
use sigil::prelude::*;

fn main() -> AppResult<()> {
    let config = AppConfig::new("Notes").size(900, 600);
    sigil::run(config, || text("Hello"))
}
# }
# fn main() {}
```

# Errors

`sigil::run` returns `ErrorKind::AlreadyRun` when the window host is started twice. With no GPU adapter the window host falls back to the CPU renderer instead of failing.

See also: [`Host`](crate::Host), [GUI and TUI](https://github.com/manaforged/sigil/blob/main/docs/guide/hosts.md).
