# Web host

The window host also builds for `wasm32-unknown-unknown`. `sigil::run` draws the same widget tree into a `<canvas>` with WebGPU. If the browser has no WebGPU, the CPU renderer draws into a 2D canvas.

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

Any example builds without changes. Load the module from a page and serve the directory over HTTP:

```html
<canvas id="app" style="width: 100%; height: 100vh"></canvas>
<script type="module">
  import init from "./hello-counter.js";
  init();
</script>
```

## Website catalog

Generate the captures and live bundle from the same clean, committed Sigil checkout. Set `sigil_site_data` to the catalog output directory:

```bash
sigil_site_data=/path/to/sigil-site-data
cargo run --locked -p sigil-ui --example visual_catalog --features terminal,desktop \
  -- --site "$sigil_site_data"
cargo build --locked --release -p sigil-ui --target wasm32-unknown-unknown \
  --example catalog_live
wasm-bindgen --target web --out-dir "$sigil_site_data/live" \
  "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/examples/catalog_live.wasm"
```

Keep the complete `wasm-bindgen` output, including its `snippets` directory. Use the matching CLI described above.

The manifest records `sigil_commit`. The website builder requires that revision to resolve to the configured Sigil repository's current `HEAD`, with no tracked source changes. An unresolved revision, an older revision, or a dirty marker requires regeneration. If you set `SIGIL_COMMIT`, use the checkout's full commit hash.

Each component records the catalog's `live_mode`: `sandbox` or `off`. A component with captures and `sandbox` requires live entries and the `catalog_live` bundle. Components with `off`, or without captures, can omit live entries. Each selectable live state records its `capture_id`; this refers to the corresponding captured state even when its display ID differs from the live state.

## Usage

```rust,no_run
# #[cfg(feature = "desktop")]
# mod web {
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("App").canvas("app"), app_tree)
}
# fn app_tree() -> impl IntoNode {
#     text("app")
# }
# }
# fn main() {}
```

`AppConfig::canvas(id)` draws into the `<canvas>` with that id and keeps its CSS size. Without it, or if no canvas has that id, `run` appends a canvas of `AppConfig::size` to `<body>`. Native hosts ignore it, so the same `main` builds for every target.

A canvas named with `AppConfig::canvas` does not take focus when the app starts, so the page keeps its own focus. Tab and Shift+Tab move between the app's widgets and leave the canvas at either end of its focus order, so keyboard users can move through the rest of the page.

`run` returns when the browser event loop starts. The GPU device arrives later, and the first frame paints when it is ready. If the browser loses the device, the next frame requests a new one and paints when it arrives. `run` writes launch and frame errors to the browser console.

## Clipboard

Copy writes through `navigator.clipboard` and returns `Delivery::Requested`. The browser has no synchronous read, so paste from the system clipboard does not reach widgets.

## Fonts

The bundled faces cover Latin, Greek, Cyrillic, and emoji. The web host cannot read system fonts. Chinese, Japanese, Korean, and other scripts outside the bundled faces draw as empty boxes until you load a font that covers them.

Load a font file from your server with `sigil::text::load_font_url`. It fetches the file, adds it as a fallback face, and lays the text out again when the face is ready:

```rust,ignore
use sigil::tasks::Status;

let cjk = sigil::text::load_font_url("fonts/NotoSansJP-Regular.otf", 0);
if let Status::Err(error) = cjk.get() {
    web_sys::console::error_1(&error.to_string().into());
}
```

Call it while you build the tree. Serve a Noto Sans JP, SC, or KR file (or a subset of it) from the same origin, or from a server that sends CORS headers. The file must be OpenType or TrueType. WOFF2 does not load. `face_index` selects a face in a `.ttc` collection.

Text in the loaded script shows empty boxes until the signal turns `Ok(())`. If it turns `Err`, the boxes stay.

## Script fallbacks

The first time text uses CJK, Arabic, Hebrew, Devanagari, Khmer, Thai, or Lao, Sigil fetches the faces listed for that script by `sigil::text::web_fallbacks()` through `load_font_url`. Sigil fetches each script once per page, and fetches nothing for scripts the text does not use. The text draws as empty boxes until the fetch completes, and stays that way if the fetch fails.

The default list is Noto Sans from jsDelivr. Each URL pins an exact commit of `notofonts/notofonts.github.io` or `notofonts/noto-cjk`, so the files do not change when those repositories change.

Call `set_web_fallbacks` before `sigil::run` to serve faces from your own origin, for example when the page is offline or its content security policy blocks the CDN. Pass an empty list to fetch nothing:

```rust,ignore
use sigil::text::{WebFont, WebFontScript, set_web_fallbacks};

set_web_fallbacks([WebFont::new(
    WebFontScript::Cjk,
    "fonts/NotoSansJP-Regular.otf",
    0,
)]);
set_web_fallbacks([]);
```

Native targets ignore this list and use the system fonts.

## Limits

- IME input goes through a hidden `<textarea>` that Sigil appends to `<body>`. It takes focus while a text input has focus and sits at the caret, so the browser draws the candidate window there. Composition events become the same `ImeEvent`s as on the window host. Other keys pass to the canvas unchanged.
- No system fonts. See [Script fallbacks](#script-fallbacks).
- Tasks run on the page's one thread. `task`, `spawn`, and `sleep` work, and `sleep` waits on `setTimeout`, but a future that computes without awaiting freezes the page until it returns.
- No WebGL. A browser without WebGPU uses the CPU renderer.
- No accessibility tree. AccessKit has no browser adapter.
