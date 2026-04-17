//! Windowed event-routing proof.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
// hello-events example: colour index is bounded by construction
// (modulo COLORS.len()). See docs/plans/native-ui/hygiene-cleanup-framework.md.
#![allow(clippy::indexing_slicing)]
#![allow(missing_docs)]

#[cfg(windows)]
use windows_core as _;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use sigil_core::prelude::*;
use sigil_events::prelude::*;
use sigil_hal::{Color, PhysicalSize, Rect, Renderer, Shell};
use sigil_layout::prelude::*;
use sigil_renderer::{WgpuRenderer, WgpuSurface, into_window_handle};
use sigil_shell::WinitShell;
use wgpu::util::DeviceExt as _;

const COLORS: [Color; 2] = [
    Color::rgba(0.04, 0.06, 0.08, 1.0),
    Color::rgba(0.20, 0.42, 0.34, 1.0),
];

#[allow(clippy::too_many_lines)]
fn main() {
    let color_index = Rc::new(Cell::new(0_usize));
    let exit_requested = Rc::new(Cell::new(false));

    let mut shell = WinitShell::new().expect("shell");
    let window_handle = shell
        .create_window("Sigil - hello-events", PhysicalSize::new(720, 480))
        .expect("create window");

    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(720.0, 480.0))
            .keyboard_shortcut(Key::Escape)
            .on_key(Key::Escape, {
                let exit_requested = Rc::clone(&exit_requested);
                move |_| exit_requested.set(true)
            })
            .child(hit_region(Rc::clone(&color_index)))
            .child(hit_region(Rc::clone(&color_index))),
    );
    let left_region = cx.nth_child(root, 0).expect("left region");
    let right_region = cx.nth_child(root, 1).expect("right region");

    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(720.0, 480.0))
        .expect("layout computes");

    let mut router = EventRouter::new();
    let mut renderer = WgpuRenderer::new().expect("renderer");
    let mut surface_slot: Option<WgpuSurface> = None;

    shell
        .run(move |shell| {
            ensure_surface(shell, window_handle, &mut renderer, &mut surface_slot);
            let mut drain_target = DrainTarget {
                router: &mut router,
                cx: &mut cx,
                layout: &layout,
                renderer: &mut renderer,
                surface_slot: &mut surface_slot,
            };
            drain_events(shell, &mut drain_target);
            if exit_requested.get() {
                shell.request_exit();
            }
            let focus_rect = focus_rect(&router, &layout, left_region, right_region);
            render(
                &mut renderer,
                &mut surface_slot,
                COLORS[color_index.get()],
                focus_rect,
            );
        })
        .expect("event loop");
}

fn hit_region(color_index: Rc<Cell<usize>>) -> NodeBuilder {
    container()
        .focusable(true)
        .layout(LayoutStyle::block().fixed_size(360.0, 480.0))
        .on_pointer_down(move |_| color_index.set((color_index.get() + 1) % COLORS.len()))
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
    let surface = renderer
        .create_surface(
            handle,
            PhysicalSize::new(size.width.max(1), size.height.max(1)),
        )
        .expect("create surface");
    *surface_slot = Some(surface);
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

fn focus_rect(
    router: &EventRouter,
    layout: &LayoutEngine,
    left: NodeId,
    right: NodeId,
) -> Option<Rect> {
    match router.focus() {
        Some(id) if id == left || id == right => layout.result(id).map(|result| result.rect()),
        _ => None,
    }
}

fn render(
    renderer: &mut WgpuRenderer,
    surface_slot: &mut Option<WgpuSurface>,
    clear: Color,
    focus_rect: Option<Rect>,
) {
    let Some(surface) = surface_slot.as_mut() else {
        return;
    };
    let size = surface.size();
    let format = surface.format();
    if let Err(err) = renderer.render_frame(surface, clear, |device, queue, view| {
        draw_focus(
            &OverlayTarget {
                device,
                queue,
                view,
                size,
                format,
            },
            focus_rect,
        );
    }) {
        eprintln!("render error: {err}");
    }
}

struct OverlayTarget<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    view: &'a wgpu::TextureView,
    size: PhysicalSize,
    format: wgpu::TextureFormat,
}

fn draw_focus(target: &OverlayTarget<'_>, rect: Option<Rect>) {
    let Some(rect) = rect else {
        return;
    };
    let vertices = focus_vertices(rect, target.size);
    let pipeline = overlay_pipeline(target.device, target.format);
    let buffer = target
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hello-events focus vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    let mut encoder = target
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("hello-events overlay"),
        });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hello-events focus pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view,
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
    target.queue.submit(core::iter::once(encoder.finish()));
}

fn overlay_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("hello-events overlay shader"),
        source: wgpu::ShaderSource::Wgsl(OVERLAY_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hello-events overlay layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hello-events overlay pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[OverlayVertex::layout()],
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
struct OverlayVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl OverlayVertex {
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

#[allow(clippy::cast_precision_loss)]
fn focus_vertices(rect: Rect, size: PhysicalSize) -> Vec<OverlayVertex> {
    let width = size.width.max(1) as f32;
    let height = size.height.max(1) as f32;
    let dims = SurfaceDims { width, height };
    let left = rect.origin.x;
    let top = rect.origin.y;
    let right = left + rect.width;
    let bottom = top + rect.height;
    let thickness = 2.0;
    let mut vertices = Vec::with_capacity(24);
    push_quad(
        &mut vertices,
        Quad::new(left, top, right, top + thickness),
        dims,
    );
    push_quad(
        &mut vertices,
        Quad::new(left, bottom - thickness, right, bottom),
        dims,
    );
    push_quad(
        &mut vertices,
        Quad::new(left, top, left + thickness, bottom),
        dims,
    );
    push_quad(
        &mut vertices,
        Quad::new(right - thickness, top, right, bottom),
        dims,
    );
    vertices
}

fn push_quad(vertices: &mut Vec<OverlayVertex>, quad: Quad, dims: SurfaceDims) {
    let color = [0.95, 0.98, 1.0, 1.0];
    let a = vertex(quad.left, quad.top, dims, color);
    let b = vertex(quad.right, quad.top, dims, color);
    let c = vertex(quad.right, quad.bottom, dims, color);
    let d = vertex(quad.left, quad.bottom, dims, color);
    vertices.extend([a, b, c, a, c, d]);
}

fn vertex(x: f32, y: f32, dims: SurfaceDims, color: [f32; 4]) -> OverlayVertex {
    OverlayVertex {
        position: [
            (x / dims.width).mul_add(2.0, -1.0),
            (y / dims.height).mul_add(-2.0, 1.0),
        ],
        color,
    }
}

#[derive(Clone, Copy)]
struct SurfaceDims {
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct Quad {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Quad {
    const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
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

const OVERLAY_SHADER: &str = r"
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
