#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

//! Repro: clicking another file in the Explorer swaps the Editor subtree
//! (a `scroll` wrapping a wide single-text-node column) via
//! `children_reactive`, against a PERSISTENT engine — exactly the shell
//! path. Goal: catch whatever "breaks the whole thing" on the second file.

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;

use sigil::prelude::*;
use sigil_hal::{
    Color, ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use sigil_renderer::{OffscreenTarget, WgpuRenderer, emit_scene};
use wgpu::TextureFormat;

const VIEWPORT: Size = Size::fixed(1440.0, 900.0);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        // Approximate a monospace block: width by longest line, height by
        // line count — so the editor's single text node reports a realistic
        // (tall) intrinsic size, the way the real shaper would.
        let text = content.as_static_str().unwrap_or("");
        let lines = text.lines().count().max(1);
        let cols = text.lines().map(str::len).max().unwrap_or(0).max(1);
        sigil_hal::LogicalSize::new(cols as f32 * 8.0, lines as f32 * 16.8)
    }
}

fn editor_view(name: &str, body: &str) -> NodeBuilder {
    let line_count = body.lines().count().max(1);
    let extent = (line_count as f32).mul_add(16.8, 20.0);
    let code = container()
        .layout(LayoutStyle::flex_column().padding(10.0).width(2000.0))
        .child(text(body.to_owned()).class("text-sm font-mono"));
    container()
        .layout(LayoutStyle::flex_column().grow(1.0))
        .class("w-full")
        .child(
            container()
                .label(name.to_owned())
                .layout(LayoutStyle::flex_row().height(28.0).padding_x(10.0)),
        )
        .child(scroll(code, extent).width(580.0).height(720.0))
}

fn recompute(layout: &mut LayoutEngine, cx: &mut Cx, root: NodeId) {
    layout.compute(cx, root, VIEWPORT).expect("layout computes");
}

const FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// Promote the renderer to a headless device and clone its `(device, queue)`
/// (both Arc-backed handles) so the offscreen render targets the renderer's OWN
/// device — the only way to hit the persistent `walk_with_ready` path.
fn renderer_device(renderer: &mut WgpuRenderer) -> Option<(wgpu::Device, wgpu::Queue)> {
    renderer.ensure_headless_for_tests().ok()?;
    let device = renderer.device()?.clone();
    let queue = renderer.queue()?.clone();
    Some((device, queue))
}

const W: u32 = 1440;
const H: u32 = 900;

/// Render the current tree through a real headless device, read pixels back, and
/// save a PNG to /tmp for inspection. The renderer is PERSISTENT across calls —
/// mirroring the live app, where the same `WgpuRenderer` (glyph cache, atlas,
/// pipeline state) is reused frame to frame.
fn render(
    renderer: &mut WgpuRenderer,
    cx: &Cx,
    root: NodeId,
    dev: Option<&(wgpu::Device, wgpu::Queue)>,
    label: &str,
) {
    let scene = emit_scene(cx, root);
    let Some((device, queue)) = dev else {
        return;
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: (W as f32, H as f32),
                scale: 1.0,
            },
            &scene,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )
        .expect("render_command_scene_offscreen");

    let raw = read_pixels(device, queue, &texture);
    let img = image::RgbaImage::from_raw(W, H, raw).expect("rgba");
    img.save(format!("/tmp/swap_{label}.png")).expect("save");
}

fn read_pixels(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let unpadded = W * 4;
    let padded = unpadded.div_ceil(align) * align;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded) * u64::from(H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(enc.finish()));
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let mapped = slice.get_mapped_range();
    let mut out = Vec::with_capacity((unpadded * H) as usize);
    for row in 0..H {
        let start = (row * padded) as usize;
        out.extend_from_slice(&mapped[start..start + unpadded as usize]);
    }
    out
}

#[test]
fn clicking_second_file_swaps_editor_without_breaking() {
    sigil::reactive::Runtime::scope("editor-swap-probe", || {
        let files = [
            (
                "short.rs".to_owned(),
                "fn main() {}\nlet x = 1;\n".to_owned(),
            ),
            (
                "long.rs".to_owned(),
                (0..120)
                    .map(|i| format!("// line {i} of a much longer source file here"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ];
        let selected = signal(0_usize);

        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0))
                .children_reactive(move || {
                    let (name, body) = &files[selected.get()];
                    vec![editor_view(name, body)]
                }),
        );

        // Render through the renderer's OWN device so the offscreen call takes
        // the PERSISTENT `walk_with_ready` path (same glyph atlas, pipeline
        // caches, and layer pool reused across frames) — exactly like the live
        // app. Requesting a separate device silently forced the temporary-
        // pipeline path, which rebuilds all GPU state per frame and hid any
        // cross-frame corruption.
        let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
        let dev = renderer_device(&mut renderer);

        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-0-initial");

        // The click: Explorer writes the new index. Next compute flushes the
        // reactive rebuild — old editor_view (with its scroll widget) removed,
        // new one built — against the SAME engine + SAME renderer. Then re-render.
        selected.set(1);
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-1-after-swap");

        // And back again, to shake out node-id recycling / stale owners.
        selected.set(0);
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-0-again");

        selected.set(1);
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-1-again");
    });
}

fn fresh_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

/// A shell-like tree: a focusable, clickable Explorer (writes `selected`) beside
/// a reactive Editor. Clicking a row rebuilds BOTH panes — the click handler's
/// own row included — which is what the live app does and the direct-signal repro
/// did not exercise.
fn shell_root(selected: Signal<usize>, files: Vec<(String, String)>) -> NodeBuilder {
    let explorer_files = files.clone();
    container()
        .layout(LayoutStyle::flex_row().grow(1.0))
        .child(
            container()
                .layout(LayoutStyle::flex_column().width(248.0).padding(6.0))
                .children_reactive(move || {
                    let active = selected.get();
                    explorer_files
                        .iter()
                        .enumerate()
                        .map(|(idx, (name, _))| explorer_row(selected, idx, name, idx == active))
                        .collect::<Vec<_>>()
                }),
        )
        .child(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0))
                .children_reactive(move || {
                    let (name, body) = &files[selected.get()];
                    vec![editor_view(name, body)]
                }),
        )
}

fn explorer_row(selected: Signal<usize>, idx: usize, name: &str, active: bool) -> NodeBuilder {
    let class = if active { "bg-accent" } else { "" };
    container()
        .role(Role::Button)
        .label(name.to_owned())
        .focusable(true)
        .layout(LayoutStyle::flex_row().height(22.0).padding_x(8.0))
        .class(class)
        .pointer_down(move |ev| {
            selected.set(idx);
            ev.stop_propagation();
        })
        .child(label(name.to_owned()))
}

#[test]
fn routed_clicks_through_explorer_dont_break() {
    sigil::reactive::Runtime::scope("editor-swap-routed", || {
        let files = vec![
            ("a.rs".to_owned(), "fn a() {}\n".to_owned()),
            (
                "b.rs".to_owned(),
                (0..90)
                    .map(|i| format!("// row {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            ("c.rs".to_owned(), "const C: u8 = 3;\n".to_owned()),
        ];
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(shell_root(selected, files));

        let mut renderer = WgpuRenderer::new().expect("renderer");
        let dev = renderer_device(&mut renderer);
        let mut layout = fresh_engine();
        let mut router = EventRouter::new();
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "routed-0");

        // Row y-centres: padding 6 + idx*22 + 11. Row1 ≈ 39, row2 ≈ 61, row0 ≈ 17.
        for (label_name, y) in [("routed-1", 39.0), ("routed-2", 61.0), ("routed-0b", 17.0)] {
            router.route(&mut cx, &layout, pointer_move(20.0, y));
            router.route(&mut cx, &layout, pointer_down(20.0, y));
            recompute(&mut layout, &mut cx, root);
            render(&mut renderer, &cx, root, dev.as_ref(), label_name);
        }
    });
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Up,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

/// The workflow's "confirmed" theory: capture the editor's scroll THUMB, then
/// rebuild the editor (disposing that scroll widget + its signals), then route a
/// pointer move/up. If the router dispatches the captured event to the disposed
/// thumb, scroll's `pointer_move` handler peeks a disposed signal and PANICS.
/// This test asserts that sequence does NOT crash.
#[test]
fn captured_scroll_thumb_survives_editor_rebuild() {
    sigil::reactive::Runtime::scope("editor-swap-capture", || {
        let files = vec![
            (
                "a.rs".to_owned(),
                (0..90)
                    .map(|i| format!("// a {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "b.rs".to_owned(),
                (0..90)
                    .map(|i| format!("// b {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ];
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(shell_root(selected, files));
        let mut layout = fresh_engine();
        let mut router = EventRouter::new();
        recompute(&mut layout, &mut cx, root);

        // The scroll thumb sits in the editor pane's right gutter. shell_root:
        // explorer 248 wide, editor at x≈248, scroll 580 wide, thumb at
        // left inset 580-8=572 -> x≈248+572=820, near the top of the scroll
        // (filename tab is 28 tall, so y≈28+thumb_top).
        for tx in [820.0_f32, 824.0, 816.0, 818.0] {
            for ty in [34.0_f32, 40.0, 30.0, 50.0] {
                router.route(&mut cx, &layout, pointer_move(tx, ty));
                router.route(&mut cx, &layout, pointer_down(tx, ty)); // thumb captures
                router.route(&mut cx, &layout, pointer_move(tx, ty + 40.0)); // drag
            }
        }

        // Rebuild the editor: removes + disposes the captured scroll widget.
        selected.set(1);
        recompute(&mut layout, &mut cx, root);

        // Captured pointer move/up AFTER disposal — the alleged crash point.
        router.route(&mut cx, &layout, pointer_move(824.0, 120.0));
        router.route(&mut cx, &layout, pointer_up(824.0, 120.0));
        recompute(&mut layout, &mut cx, root);
        // Swap back and forth a few times under a held capture, too.
        for s in [0_usize, 1, 0, 1] {
            router.route(&mut cx, &layout, pointer_down(824.0, 40.0));
            selected.set(s);
            recompute(&mut layout, &mut cx, root);
            router.route(&mut cx, &layout, pointer_up(824.0, 120.0));
        }
    });
}

/// Regression for the "long file makes the editor escape to the window's
/// top-left, painting over the sidebar" bug. A deep reactive subtree
/// (`pane > children_reactive(scroll(tall))`) inside a BIG tree (so the
/// editor-switch dirty set is < half the nodes → the engine takes the
/// `incremental_sync_and_measure` path) used to spawn two containment roots —
/// the tree root AND the scroll's nested fixed-size box — and the nested one
/// was remeasured as an isolated root anchored at (0,0), clobbering the
/// content's real in-tree position. Asserts the tall content stays placed.
#[test]
// Upstream test walks one toggle-remeasure scenario (53 lines);
// splitting it would obscure the regression it pins.
#[allow(clippy::too_many_lines)]
fn deep_reactive_scroll_switch_stays_positioned() {
    sigil::reactive::Runtime::scope("deep-reactive-scroll", || {
        let tall = signal(false);
        let pane = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .children_reactive(move || {
                let lines = if tall.get() { 200 } else { 2 };
                let body = (0..lines)
                    .map(|i| format!("line {i} of source"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let extent = (lines as f32).mul_add(16.8, 20.0);
                let code = container()
                    .layout(LayoutStyle::flex_column().padding(10.0).width(2000.0))
                    .child(text(body));
                vec![scroll(code, extent).width(580.0).height(720.0)]
            });
        // Filler inflates the live node count so the editor-switch dirty set
        // stays under half → the incremental layout path (where the bug lives).
        let mut filler = container().layout(LayoutStyle::flex_column());
        for i in 0..120 {
            filler = filler.child(label(format!("filler row {i}")));
        }
        let tree = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .child(filler)
            .child(
                container()
                    .layout(LayoutStyle::flex_row().grow(1.0))
                    .child(
                        container()
                            .layout(LayoutStyle::flex_column().width(248.0))
                            .child(label("SIDEBAR".to_owned())),
                    )
                    .child(pane),
            );
        let mut cx = Cx::new_root();
        let root = cx.add(tree);
        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);

        tall.set(true); // open the "long file"
        recompute(&mut layout, &mut cx, root);

        let mut escaped = Vec::new();
        cx.visit_preorder(root, |id, _node| {
            if let Some(res) = layout.result(id) {
                let r = res.rect();
                // Width targets the scroll body (`width(2000)`), not the viewport-width
                // filler column, which now legitimately sits tall (auto-height column
                // sizes to content, not shrinks).
                if r.height > 900.0 && r.width > 1500.0 && r.origin.x < 60.0 && r.origin.y < 60.0 {
                    escaped.push((r.origin.x, r.origin.y, r.width, r.height));
                }
            }
        });
        assert!(
            escaped.is_empty(),
            "tall scroll content escaped to the top-left (incremental containment-root bug): {escaped:?}"
        );
    });
}
