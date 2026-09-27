# Web host

The window host also builds for `wasm32-unknown-unknown`. `sigil::run`
draws the same widget tree into a `<canvas>` with WebGPU. If the browser
has no WebGPU, the CPU renderer draws into a 2D canvas.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --example hello-counter
wasm-bindgen --target web --out-dir web \
  target/wasm32-unknown-unknown/release/examples/hello-counter.wasm
```

Use the `wasm-bindgen` CLI version that `Cargo.lock` pins. Any example
builds without changes. Load the module from a page and serve the
directory over HTTP:

```html
<canvas id="app" style="width: 100%; height: 100vh"></canvas>
<script type="module">
  import init from "./hello-counter.js";
  init();
</script>
```

## Usage

```rust,ignore
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("App").canvas("app"), app_tree)
}
```

`AppConfig::canvas(id)` draws into the `<canvas>` with that id and keeps
its CSS size. Without it, or if no canvas has that id, `run` appends a
canvas of `AppConfig::size` to `<body>`. Native hosts ignore it, so the
same `main` builds for every target.

`run` returns when the browser event loop starts. The GPU device
arrives later, and the first frame paints when it is ready. If the
browser loses the device, the next frame requests a new one and paints
when it arrives. `run` writes launch and frame errors to the browser
console.

## Clipboard

Copy writes through `navigator.clipboard` and returns
`Delivery::Requested`. The browser has no synchronous read, so paste
from the system clipboard does not reach widgets.

## Limits

- No IME. The winit web backend sends no composition events, so
  CJK input does not reach text fields. Keys and typed Latin text work.
- No system fonts. Scripts outside the bundled faces do not draw.
- `spawn` and `sleep` futures do not run. The async runtime has no
  browser executor.
- No WebGL. A browser without WebGPU uses the CPU renderer.
- No accessibility tree. AccessKit has no browser adapter.
