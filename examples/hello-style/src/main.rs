//! Windowed style proof.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
#![allow(missing_docs)]

#[cfg(windows)]
use windows_core as _;

use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;

use sigil_async as _;
use sigil_core::prelude::*;
use sigil_events::prelude::*;
use sigil_hal::{Color, PhysicalSize, PointerEvent, Renderer, Shell};
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};
use sigil_renderer::{SceneItem, WgpuRenderer, WgpuSurface, emit_scene, into_window_handle};
use sigil_shell::WinitShell;
use sigil_style::{NodeStyleExt, StyleAtom};
use wgpu::util::DeviceExt as _;

const APP_WIDTH: f32 = 720.0;
const APP_HEIGHT: f32 = 480.0;
const OUTER_PAD: f32 = 16.0;
const GAP: f32 = 16.0;
const PANE_WIDTH: f32 = 328.0;
const TOP_HEIGHT: f32 = 304.0;
const BOTTOM_HEIGHT: f32 = 128.0;
const RIGHT_LEFT: f32 = OUTER_PAD + PANE_WIDTH + GAP;
const RIGHT_RIGHT: f32 = RIGHT_LEFT + PANE_WIDTH;
const TOP_BOTTOM: f32 = OUTER_PAD + TOP_HEIGHT;

static HOVER_OFF: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static HOVER_ON: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];

fn main() -> Result<(), Box<dyn Error>> {
    Runtime::scope("hello-style", run)
}

fn run() -> Result<(), Box<dyn Error>> {
    let hover = signal(false);
    let exit_requested = Rc::new(Cell::new(false));

    let mut shell = WinitShell::new()?;
    let window_handle = shell.create_window("Sigil - hello-style", PhysicalSize::new(720, 480))?;

    let mut cx = Cx::new_root();
    let root = cx.add(app_tree(hover, Rc::clone(&exit_requested)));

    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout.compute(&mut cx, root, Size::fixed(APP_WIDTH, APP_HEIGHT))?;

    let mut router = EventRouter::new();
    let mut renderer = WgpuRenderer::new()?;
    let mut surface_slot: Option<WgpuSurface> = None;

    shell.run(move |shell| {
        ensure_surface(shell, window_handle, &mut renderer, &mut surface_slot);
        let mut target = DrainTarget {
            router: &mut router,
            cx: &mut cx,
            layout: &layout,
            renderer: &mut renderer,
            surface_slot: &mut surface_slot,
        };
        drain_events(shell, &mut target);
        if exit_requested.get() {
            shell.request_exit();
        }
        render(&cx, root, &mut renderer, &mut surface_slot);
    })?;

    Ok(())
}

fn app_tree(hover: sigil_reactive::Signal<bool>, exit_requested: Rc<Cell<bool>>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(APP_WIDTH, APP_HEIGHT)
                .padding_px(OUTER_PAD)
                .gap_px(GAP),
        )
        .keyboard_shortcut(Key::Escape)
        .on_key(Key::Escape, move |_| exit_requested.set(true))
        // Stopgap until pointer-enter / pointer-leave phases exist:
        // root capture maps pointer position into the right pane bounds.
        .on_pointer_move_capture(move |event| hover.set(in_hover_pane(event)))
        .child(top_row(hover))
        .child(
            card()
                .layout(LayoutStyle::block().fixed_size(688.0, BOTTOM_HEIGHT))
                .class("bg-accent"),
        )
}

fn top_row(hover: sigil_reactive::Signal<bool>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(688.0, TOP_HEIGHT)
                .gap_px(GAP),
        )
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(PANE_WIDTH, TOP_HEIGHT))
                .class("p-8 bg-accent rounded-md"),
        )
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(PANE_WIDTH, TOP_HEIGHT))
                .class("p-8 rounded-md")
                .style_reactive(move || if hover.get() { HOVER_ON } else { HOVER_OFF }),
        )
}

fn card() -> NodeBuilder {
    container().class("p-4 bg-surface rounded-md")
}

fn in_hover_pane(event: &PointerEvent) -> bool {
    let point = event.position;
    point.x >= RIGHT_LEFT && point.x < RIGHT_RIGHT && point.y >= OUTER_PAD && point.y < TOP_BOTTOM
}

fn ensure_surface(
    shell: &WinitShell,
    window_handle: sigil_shell::WinitWindowHandle,
    renderer: &mut WgpuRenderer,
    surface_slot: &mut Option<WgpuSurface>,
) {
    if surface_slot.is_some() {
        return;
    }
    let Some(window) = shell.window_for(window_handle) else {
        return;
    };
    let size = window.inner_size();
    let handle = into_window_handle(Arc::clone(window));
    match renderer.create_surface(
        handle,
        PhysicalSize::new(size.width.max(1), size.height.max(1)),
    ) {
        Ok(surface) => *surface_slot = Some(surface),
        Err(err) => eprintln!("create surface error: {err}"),
    }
}

struct DrainTarget<'a> {
    router: &'a mut EventRouter,
    cx: &'a mut Cx,
    layout: &'a LayoutEngine,
    renderer: &'a mut WgpuRenderer,
    surface_slot: &'a mut Option<WgpuSurface>,
}

fn drain_events(shell: &mut WinitShell, target: &mut DrainTarget<'_>) {
    for event in shell.drain_events() {
        if let ShellEvent::Window(WindowEvent::Resized(size)) = &event
            && let Some(surface) = target.surface_slot.as_mut()
        {
            let _ = target.renderer.resize(surface, *size);
        }
        target.router.route(target.cx, target.layout, event);
    }
}

fn render(
    cx: &Cx,
    root: NodeId,
    renderer: &mut WgpuRenderer,
    surface_slot: &mut Option<WgpuSurface>,
) {
    let Some(surface) = surface_slot.as_mut() else {
        return;
    };
    let scene = emit_scene(cx, root);
    let format = surface.format();
    if let Err(err) = renderer.render_frame(
        surface,
        Color::rgba(0.03, 0.04, 0.05, 1.0),
        |device, queue, view| draw_scene(device, queue, view, format, &scene),
    ) {
        eprintln!("render error: {err}");
    }
}

fn draw_scene(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    format: wgpu::TextureFormat,
    scene: &sigil_renderer::Scene,
) {
    let vertices = scene_vertices(scene);
    if vertices.is_empty() {
        return;
    }
    let pipeline = rect_pipeline(device, format);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("hello-style rect vertices"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("hello-style rect encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hello-style rect pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..u32::try_from(vertices.len()).unwrap_or(0), 0..1);
    }
    queue.submit(core::iter::once(encoder.finish()));
}

fn scene_vertices(scene: &sigil_renderer::Scene) -> Vec<RectVertex> {
    let mut vertices = Vec::new();
    for item in scene.items() {
        if let SceneItem::Rect(rect) = item {
            push_rect(&mut vertices, rect);
        }
    }
    vertices
}

fn push_rect(vertices: &mut Vec<RectVertex>, rect: &sigil_renderer::SceneRect) {
    let color = rect_color(rect);
    let left = rect.rect.origin.x;
    let top = rect.rect.origin.y;
    let right = left + rect.rect.width;
    let bottom = top + rect.rect.height;
    let a = vertex(left, top, color);
    let b = vertex(right, top, color);
    let c = vertex(right, bottom, color);
    let d = vertex(left, bottom, color);
    vertices.extend([a, b, c, a, c, d]);
}

#[allow(clippy::cast_precision_loss)]
fn rect_color(rect: &sigil_renderer::SceneRect) -> [f32; 4] {
    let color = rect.color;
    [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        (f32::from(color.a) / 255.0) * (f32::from(rect.opacity) / 10_000.0),
    ]
}

fn vertex(x: f32, y: f32, color: [f32; 4]) -> RectVertex {
    RectVertex {
        position: [
            (x / APP_WIDTH).mul_add(2.0, -1.0),
            (y / APP_HEIGHT).mul_add(-2.0, 1.0),
        ],
        color,
    }
}

fn rect_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hello-style rect shader"),
        source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hello-style rect layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hello-style rect pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[RectVertex::layout()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl RectVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: u64::try_from(core::mem::size_of::<Self>()).unwrap_or(0),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRS,
        }
    }
}

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(
        &self,
        _content: &TextContent,
        _space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        taffy::prelude::Size::ZERO
    }
}

const RECT_SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}
";
