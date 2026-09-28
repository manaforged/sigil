# Fuzz targets

Five libFuzzer targets cover the parsers that read text input. `scene_render`
covers the CPU renderer.

| Target | Parser |
| --- | --- |
| `html` | `widgets::html` |
| `markdown` | `widgets::markdown` |
| `mermaid` | `widgets::mermaid` |
| `style_class` | `style::parse_class_string`, `style::parse_stateful_token` |
| `svg_icon` | `icons::IconBaker::bake` |
| `scene_render` | `renderer::CpuRenderer::render` on scenes from `sigil_kittest::scene_gen::generate` |

## Requirements

Nightly Rust and `cargo-fuzz`:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

## Run

From the repository root:

```bash
cargo +nightly fuzz build
cargo +nightly fuzz run <target> -- -max_total_time=600 -rss_limit_mb=2048 -timeout=10
```

Add `-fork=<n>` to run `n` jobs of one target in parallel.

## Crashes

libFuzzer writes a failing input to `fuzz/artifacts/<target>/`. Minimize it and replay it:

```bash
cargo +nightly fuzz tmin <target> fuzz/artifacts/<target>/<file>
cargo +nightly fuzz run <target> <file>
```

Fix the defect in the parser, then copy the minimized input to `fuzz/regressions/<target>/`.

## Regressions

`fuzz/regressions/<target>/` holds minimized inputs that failed before a fix. Replay them:

```bash
cargo +nightly fuzz run <target> fuzz/regressions/<target>/* -- -runs=0
```
