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

The `wasm-bindgen` CLI must match the version that `Cargo.lock` pins:

```bash
cargo install wasm-bindgen-cli --version "$(cargo pkgid wasm-bindgen | cut -d@ -f2)"
```

Any example builds without changes. Load the module from a page and serve the
directory over HTTP:

```html
<canvas id="app" style="width: 100%; height: 100vh"></canvas>
<script type="module">
  import init from "./hello-counter.js";
  init();
</script>
```

## Usage

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("App").canvas("app"), app_tree)
}
# fn app_tree() -> impl IntoNode {
#     text("app")
# }
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

## Fonts

The bundled faces cover Latin, Greek, Cyrillic, and emoji. The browser
host cannot read system fonts. Chinese, Japanese, Korean, and other
scripts outside the bundled faces draw as empty boxes until you load a
font that covers them.

Load a font file from your server with `sigil::text::load_font_url`.
It fetches the file, adds it as a fallback face, and lays the text out
again when the face is ready:

```rust,ignore
use sigil::tasks::Status;

let cjk = sigil::text::load_font_url("fonts/NotoSansJP-Regular.otf", 0);
if let Status::Err(error) = cjk.get() {
    web_sys::console::error_1(&error.to_string().into());
}
```

Call it while you build the tree. Serve a Noto Sans JP, SC, or KR file
(or a subset of it) from the same origin, or from a server that sends
CORS headers. The file must be OpenType or TrueType. WOFF2 does not
load. `face_index` selects a face in a `.ttc` collection.

Text in the loaded script shows empty boxes until the signal turns
`Ok(())`. If it turns `Err`, the boxes stay.

## Limits

- IME input goes through a hidden `<textarea>` that Sigil appends to
  `<body>`. It takes focus while a text input has focus and sits at the
  caret, so the browser draws the candidate window there. Composition
  events become the same `ImeEvent`s as on the desktop host. Other keys
  pass to the canvas unchanged.
- No system fonts. The first time text uses CJK, Arabic, Hebrew,
  Devanagari, Khmer, Thai, or Lao, Sigil fetches a Noto face for that
  script from jsDelivr through `load_font_url`. The text draws as empty
  boxes until the fetch completes, and stays that way if the page is
  offline or its content security policy blocks the CDN. Call
  `load_font_url` to serve a face from your own origin.
- Tasks run on the page's one thread. `task`, `spawn`, and `sleep`
  work, and `sleep` waits on `setTimeout`, but a future that computes
  without awaiting freezes the page until it returns.
- No WebGL. A browser without WebGPU uses the CPU renderer.
- No accessibility tree. AccessKit has no browser adapter.
