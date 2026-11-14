use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use bytemuck::{Pod, Zeroable};

use crate::profile::{Counter, count};
use crate::renderer::fill_paint::setup::{QuadDesc, QuadPipeline};
use crate::renderer::layer_composite::{CompositeRequest, uv_max};
use crate::renderer::scene_instance::{BlendSpace, affine_rows};
use crate::scene::command::Transform;
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shader::{COLORS, ShaderKey, Shading};

#[path = "shader_paint/prelude.rs"]
mod prelude;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Form {
    Fill,
    Layer,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ShaderInstance {
    rect: [f32; 4],
    radii: [f32; 4],
    vars: [[f32; 4]; 2],
    colors: [[f32; 4]; COLORS],
    params: [f32; 4],
    layer_uv: [f32; 4],
    xform: [[f32; 4]; 2],
}

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<ShaderInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4,
        11 => Float32x4,
        12 => Float32x4,
    ],
};

pub(crate) struct Place {
    pub(crate) rect: crate::geometry::Rect,
    pub(crate) radii: CornerRadii,
    pub(crate) opacity: f32,
    pub(crate) transform: Transform,
    pub(crate) layer_uv: [f32; 2],
}

impl ShaderInstance {
    pub(crate) fn new(place: &Place, shading: &Shading, space: BlendSpace, scale: f32) -> Self {
        let rect = place.rect;
        let v = shading.vars;
        let [layer_u, layer_v] = place.layer_uv;
        Self {
            rect: [rect.origin.x, rect.origin.y, rect.width, rect.height],
            radii: place.radii.normalized_for(rect.width, rect.height),
            vars: [[v[0], v[1], v[2], v[3]], [v[4], v[5], v[6], v[7]]],
            colors: shading.colors.map(|color| BlendSpace::Linear.rgba(color)),
            params: [
                place.opacity.clamp(0.0, 1.0),
                scale,
                f32::from(u8::from(space == BlendSpace::Gamma)),
                0.0,
            ],
            layer_uv: [layer_u, layer_v, 0.0, 0.0],
            xform: affine_rows(place.transform),
        }
    }

    pub(crate) fn layer(
        request: &CompositeRequest<'_>,
        shading: &Shading,
        space: BlendSpace,
        scale: f32,
    ) -> Self {
        let place = Place {
            rect: request.dest,
            radii: request.clip_radii,
            opacity: request.opacity,
            transform: request.transform,
            layer_uv: uv_max(request.source_extent, request.texture_size),
        };
        Self::new(&place, shading, space, scale)
    }
}

type Compiled = Pin<Box<dyn Future<Output = Option<wgpu::Error>>>>;

enum State {
    Pending(Compiled, QuadPipeline),
    Ready(QuadPipeline),
    Failed,
}

struct Program {
    key: ShaderKey,
    form: Form,
    format: wgpu::TextureFormat,
    state: State,
}

impl Program {
    fn poll(&mut self) {
        let State::Pending(future, _) = &mut self.state else {
            return;
        };
        let mut cx = Context::from_waker(Waker::noop());
        let Poll::Ready(outcome) = future.as_mut().poll(&mut cx) else {
            return;
        };
        let State::Pending(_, quad) = std::mem::replace(&mut self.state, State::Failed) else {
            return;
        };
        match outcome {
            None => self.state = State::Ready(quad),
            Some(error) => tracing::error!(
                target: "sigil::shader",
                %error,
                form = ?self.form,
                "a shader did not compile; drawing its fallback"
            ),
        }
    }
}

#[derive(Default)]
pub(crate) struct ShaderPrograms {
    programs: Vec<Program>,
}

impl ShaderPrograms {
    pub(crate) fn ready(
        &mut self,
        device: &wgpu::Device,
        (key, form, format): (ShaderKey, Form, wgpu::TextureFormat),
        groups: &[Option<&wgpu::BindGroupLayout>],
    ) -> Option<&QuadPipeline> {
        let found = self
            .programs
            .iter()
            .position(|p| p.key == key && p.form == form && p.format == format);
        let index = found.unwrap_or_else(|| {
            self.programs.push(Program {
                key,
                form,
                format,
                state: compile(device, (key, form, format), groups),
            });
            self.programs.len() - 1
        });
        let program = self.programs.get_mut(index)?;
        program.poll();
        match &program.state {
            State::Ready(quad) => Some(quad),
            State::Pending(..) | State::Failed => None,
        }
    }
}

fn compile(
    device: &wgpu::Device,
    (key, form, format): (ShaderKey, Form, wgpu::TextureFormat),
    groups: &[Option<&wgpu::BindGroupLayout>],
) -> State {
    count(Counter::GpuShaderPipelines, 1);
    let Some(source) = key.source() else {
        return State::Failed;
    };
    let wgsl = prelude::assemble(form, &source);
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let quad = QuadPipeline::new(
        device,
        format,
        QuadDesc {
            label: "sigil-renderer shader",
            wgsl: &wgsl,
            groups,
            instance: INSTANCE_LAYOUT,
            fs_entry: "fs_main",
            blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        },
    );
    State::Pending(Box::pin(scope.pop()), quad)
}
