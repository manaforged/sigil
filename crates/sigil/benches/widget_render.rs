//! Widget-render baseline bench — W5.0b infrastructure for W5.1.
//!
//! Measures cost of rendering each catalog widget in `Default` and
//! `Hover` modes through the offscreen wgpu path. After W5.1 the
//! binding-install hook may add per-frame overhead routed through
//! `Animator::tick`; this curve is the regression baseline.
//!
//! Each iteration spans: build widget → layout → drive interaction →
//! re-layout → `emit_scene` → `render_command_scene_offscreen` →
//! `device.poll`. wgpu device + renderer + capture texture are built
//! once per group and reused across iterations to keep the measured
//! cost on the per-frame hot path.
//!
//! Run with `cargo bench -p sigil --bench widget_render`. Compile-only
//! smoke: `cargo bench -p sigil --bench widget_render -- --test`.
//!
//! The bench cleanly skips when no wgpu adapter is available on the
//! host (same heuristic as the visual-golden tests).

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::too_many_arguments
)]

// Workspace deps inherited by the bench target but not directly named.
use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use events as _;
use image as _;
use kernel as _;
use layout as _;
use reactive as _;
use style as _;
use tasks as _;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hal::{
    Color, ModifiersState, MouseButton, MouseButtons, Point, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use pollster::FutureExt as _;
use renderer::{CommandScene, OffscreenTarget, WgpuRenderer, emit_scene};
use sigil::prelude::*;
use sigil::reactive::{Runtime, signal};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Which interaction state to drive the widget into.
#[derive(Clone, Copy, Debug)]
enum Mode {
    Default,
    Hover,
}

impl Mode {
    const fn tag(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Hover => "hover",
        }
    }
}

/// Stub text measurement (deterministic, matches golden_support).
struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: hal::Size) -> hal::LogicalSize {
        hal::LogicalSize::new(40.0, 16.0)
    }
}

/// Per-group GPU resources held across iterations.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: WgpuRenderer,
}

fn try_init_gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .block_on()
        .ok()?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("widget-render bench device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()?;
    let renderer = WgpuRenderer::new().ok()?;
    Some(Gpu {
        device,
        queue,
        renderer,
    })
}

fn make_capture(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("widget-render bench capture"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
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
    (texture, view)
}

fn pointer_move(x: f32, y: f32) -> ShellEvent {
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

#[allow(dead_code)]
fn pointer_down(x: f32, y: f32) -> ShellEvent {
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

fn drive(
    router: &mut EventRouter,
    cx: &mut Cx,
    layout: &LayoutEngine,
    mode: Mode,
    hit: (f32, f32),
) {
    if matches!(mode, Mode::Hover) {
        router.route(cx, layout, pointer_move(hit.0, hit.1));
    }
    // Default mode: no events.
    let _ = layout;
    let _ = cx;
}

/// One iteration: build → layout → drive → relayout → emit_scene →
/// render → device.poll. Mirrors the golden capture path minus PNG IO.
fn render_one(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut WgpuRenderer,
    view: &wgpu::TextureView,
    build: &dyn Fn(&mut Cx) -> NodeId,
    mode: Mode,
    size_px: (f32, f32),
    hit: (f32, f32),
) {
    Runtime::scope("widget-render-bench", || {
        let mut cx = Cx::new_root();
        let root = build(&mut cx);

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(size_px.0, size_px.1))
            .expect("layout");

        let mut router = EventRouter::new();
        drive(&mut router, &mut cx, &layout, mode, hit);

        layout
            .compute(&mut cx, root, Size::fixed(size_px.0, size_px.1))
            .expect("layout-after-state");

        let scene: CommandScene = emit_scene(&cx, root);
        let clear = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        renderer
            .render_command_scene_offscreen(
                OffscreenTarget {
                    device,
                    queue,
                    view,
                    format: FORMAT,
                    viewport: size_px,
                    scale: 1.0,
                },
                &scene,
                clear,
            )
            .expect("render_command_scene_offscreen");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll");
    });
}

/// Widget catalog entry. `build` is a stable `fn` pointer; bench-time
/// state (signals) is constructed afresh inside the closure each
/// iteration so the reactive runtime stays scoped to the iter.
struct Widget {
    name: &'static str,
    size_px: (u32, u32),
    hit: (f32, f32),
    build: fn(&mut Cx) -> NodeId,
}

fn build_button(cx: &mut Cx) -> NodeId {
    cx.add(button("Save"))
}

fn build_checkbox(cx: &mut Cx) -> NodeId {
    let checked = signal(false);
    cx.add(checkbox(checked).class("bg-surface rounded-sm"))
}

fn build_slider(cx: &mut Cx) -> NodeId {
    let v = signal(50.0_f32);
    cx.add(slider(v, 0.0..=100.0))
}

fn build_dropdown(cx: &mut Cx) -> NodeId {
    let sel = signal(0_usize);
    cx.add(dropdown(
        sel,
        vec!["Easy".into(), "Normal".into(), "Hard".into()],
    ))
}

fn build_progress(cx: &mut Cx) -> NodeId {
    let v = signal(50.0_f32);
    cx.add(progress(v, 100.0).height(24.0))
}

fn build_panel(cx: &mut Cx) -> NodeId {
    cx.add(panel().title("Settings"))
}

const WIDGETS: &[Widget] = &[
    Widget {
        name: "button",
        size_px: (160, 56),
        hit: (74.0, 22.0),
        build: build_button,
    },
    Widget {
        name: "checkbox",
        size_px: (36, 36),
        hit: (16.0, 16.0),
        build: build_checkbox,
    },
    Widget {
        name: "slider",
        size_px: (192, 36),
        hit: (90.0, 14.0),
        build: build_slider,
    },
    Widget {
        name: "dropdown",
        size_px: (212, 52),
        hit: (90.0, 22.0),
        build: build_dropdown,
    },
    Widget {
        name: "progress",
        size_px: (244, 32),
        hit: (1.0, 1.0),
        build: build_progress,
    },
    Widget {
        name: "panel",
        size_px: (340, 92),
        hit: (1.0, 1.0),
        build: build_panel,
    },
];

fn bench_widget_render(c: &mut Criterion) {
    let Some(Gpu {
        device,
        queue,
        mut renderer,
    }) = try_init_gpu()
    else {
        eprintln!("widget_render: no wgpu adapter — skipping");
        return;
    };

    let mut group = c.benchmark_group("widget_render");
    for w in WIDGETS {
        let (tex_w, tex_h) = w.size_px;
        let (_capture, view) = make_capture(&device, tex_w, tex_h);
        let size_f = (tex_w as f32, tex_h as f32);
        let build_ref: &dyn Fn(&mut Cx) -> NodeId = &w.build;

        for mode in [Mode::Default, Mode::Hover] {
            group.bench_function(format!("{}_{}", w.name, mode.tag()), |b| {
                b.iter(|| {
                    render_one(
                        &device,
                        &queue,
                        &mut renderer,
                        &view,
                        build_ref,
                        black_box(mode),
                        black_box(size_f),
                        black_box(w.hit),
                    );
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_widget_render);
criterion_main!(benches);
