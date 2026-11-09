One widget tree without a window: mount it, route events to it, lay it out, and paint it.

`sigil::run` drives a `Host` for you. Use one directly to embed Sigil in another engine's frame loop, or to drive a tree from a script. `HostMode::Pixel` lays out in points, and `HostMode::Cell` in terminal cells.

```rust,no_run
use sigil::Host;
use sigil::prelude::*;
use sigil::reactive::Owner;

Owner::scope("embed", || {
    let mut host = Host::new(HostMode::Pixel);
    host.mount(|| column().gap(Space::Md).child(text("Status: ok")));
    if host.layout((480.0, 240.0)).is_ok() {
        println!("{}", host.accessibility_text());
    }
});
```

# Errors

[`Host::layout`](crate::Host::layout) returns `ErrorKind::Layout` when the tree holds a stale node id.

See also: [`AppConfig`](crate::AppConfig), [Headless driving and automation](https://github.com/manaforged/sigil/blob/main/docs/guide/automation.md).
