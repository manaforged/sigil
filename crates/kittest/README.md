# sigil-kittest

Visual golden tests for Sigil widgets. The harness drives a widget into an interaction state, renders it offscreen with wgpu, reads the pixels back, and compares them with a PNG committed under your crate's `tests/goldens/` directory.

For interaction tests, `Harness` finds controls by accessible role and name, clicks and types through the event router, and compares the terminal grid or the rendered image with a golden. See the [testing guide](https://github.com/manaforged/sigil/blob/main/docs/guide/testing.md).

## Install

Add the crate as a dev-dependency:

```bash
cargo add --dev --git https://github.com/manaforged/sigil sigil-kittest
```

## Usage

```rust,no_run
use sigil::prelude::*;
use sigil_kittest::{Golden, Harness};

#[test]
fn button_default() -> Result<(), sigil_kittest::Error> {
    let Some(img) = Harness::render(|| button("Save"))
        .size([200, 60])
        .hit((10.0, 10.0))
        .image()?
    else {
        return Ok(());
    };
    Golden::new("button_default").assert_image(&img);
    Ok(())
}
```

`Harness::render(build)` returns a `Render`. Set `.mode(mode)`, `.size([w, h])`, `.scale(scale)`, and `.hit((x, y))`, then call `.image()` for `Result<Option<RgbaImage>, Error>`. The hit point must land inside the widget for `Hover`, `Pressed`, and `Clicked` to take effect. For `Disabled`, build the widget disabled, for example `button("Save").disabled(true)`; the harness sends no events in that mode. `.focus_first()` drives the first focusable descendant. `.capture(&mut settle)` also returns the top layer bounds. `.filmstrip(frames, dt)` puts hover frames side by side.

`Golden::new(stem)` resolves `tests/goldens/<stem>.png` under `CARGO_MANIFEST_DIR`, read when the test runs. `cargo test` sets it to the crate that the test belongs to. If you run a test binary directly, set it yourself, or the path is relative to the current directory. `golden.assert_differs(&other)` fails when two goldens are identical.

## Interaction modes

| Mode | Effect |
| --- | --- |
| `Default` | No input. |
| `Hover` | Pointer over the hit point, then ticks until animations settle. |
| `Pressed` | Pointer down at the hit point, then ticks until animations settle. |
| `Clicked` | Pointer down and up at the hit point, then ticks until animations settle. |
| `Focused` | Keyboard focus on the widget, then ticks until animations settle. |
| `Disabled` | No input. |
| `HoverAfterFrames`, `PressedAfterFrames`, `FocusedAfterFrames` | The same input, then `frames` animation ticks of `dt` seconds. |

`frames: u32::MAX` ticks until every animation settles. If animations are still active after `SETTLE_SAFETY_CAP` (600) frames, the harness returns an `Error` whose `kind()` is `ErrorKind::Unsettled`; `error.is_unsettled()` tests for it. `InteractionMode::tag()` returns the same tag for a mode and its `AfterFrames` form, so add your own suffix to the golden name for an animated golden.

## Text layout

The harness uses the real text engine. For a fixed 40x16 box per text node, call `.stub()`, or pass any measure with `.measure(|| ...)`.

Layout tests that need a deterministic measure without rendering pass `FakeMeasure::fixed(width, height)` (one box per text node) or `FakeMeasure::chars(char_width, line_height)` (widest line times `char_width`, line count times `line_height`) to `LayoutEngine::new`.

## No GPU

If no wgpu adapter is available, the harness returns an `Error` whose `kind()` is `ErrorKind::NoAdapter`; `error.is_no_adapter()` tests for it. Set `SIGIL_ALLOW_NO_GPU=1` to get `Ok(None)` and skip the check instead. All calls in one test process share one wgpu device (`gpu::shared_device`).

## Errors

Every fallible call returns `sigil_kittest::Error`. Branch on `error.kind()`, an `ErrorKind`, or on a predicate, and never on the message. `source()` returns the cause, such as the layout or renderer error; the message does not repeat it. A harness step that panics prints the whole chain.

## Update goldens

After an intentional rendering change:

1. Run the tests with `SIGIL_REGEN_GOLDENS=1` set. The harness writes each image to its golden path instead of comparing.
2. Open every new PNG and check that the widget renders correctly.
3. Commit the PNGs.

## Per-adapter goldens

Edge anti-aliasing differs between GPU adapters. The harness derives an adapter key from the wgpu adapter: the backend and the vendor class, for example `dx12-warp`, `dx12-amd`, `vulkan-nvidia`, or `metal-apple`. `gpu::adapter_key()` returns it.

The harness compares against `<name>.<key>.png` if that file exists. Otherwise it compares against `<name>.png`. The budget does not change.

To bake the goldens for one adapter:

1. Run the tests on that adapter with `SIGIL_REGEN_GOLDENS=adapter` set. The harness writes `<name>.<key>.png` only for a test that exceeds its budget against `<name>.png`. It deletes an adapter golden that is no longer needed.
2. Compare every new PNG with its base golden. Accept only edge anti-aliasing differences.
3. Commit the PNGs.

`SIGIL_REGEN_GOLDENS=1` writes the adapter golden when one exists for the current adapter, and the base golden otherwise.

## Tolerance

`Golden::delta(max_delta_pct)` allows a percentage of pixels to differ, to absorb GPU driver differences. A pixel differs when any RGBA channel differs by more than 2. The default is `0.5`. `Golden::assert(rendered)` takes the `Rendered` result directly and skips the check when no adapter is available.

The offscreen target clears to opaque black.

## Profile a scenario

`Harness::gpu(width, height, scale)` paints each frame through the GPU renderer and waits for the GPU. Every harness frame is a profile frame, so `sigil::profile::summary()` reports layout, emit, submit, and GPU time, plus a `why` section that names the nodes, layers, and repaints behind the cost.

```sh
cargo bench -p sigil-kittest --bench frame_scenarios
SIGIL_SCENARIO=modal_table_scroll cargo bench -p sigil-kittest --bench frame_scenarios
```

The scenarios are `page_scroll`, `modal_table_scroll`, and `hover_sweep`. `SIGIL_PROFILE_BUDGET_MS` sets the frame budget (default 8.33 ms).

## Raw captures

For renderer tests that inspect pixels, `gpu::render_capture` renders a `CommandScene` into a new texture described by `gpu::Capture`: `Capture::new(viewport, size_px)`, then `.scale(scale)` and `.clear(color)`. `gpu::read_pixels` reads it back as RGBA8 rows, and `gpu::pixel_rgba` returns one pixel. `gpu::create_capture_texture` and `gpu::FORMAT` create a target for custom render paths.

## License

[MIT](https://github.com/manaforged/sigil/blob/main/LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
