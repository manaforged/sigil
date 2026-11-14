use crate::profile::{Counter, count};
use crate::renderer::fill_paint::setup::QuadPipeline;
use crate::renderer::frame_upload::FrameUploads;
use crate::renderer::layer_composite::LayerCompositePipeline;
use crate::renderer::scene_command::walk::{SceneWalk, WalkCtx, transform_rect};
use crate::renderer::scene_instance::BlendSpace;
use crate::renderer::scene_paint::PaintCtx;
use crate::renderer::shader_paint::{Form, Place, ShaderInstance, ShaderPrograms};
use crate::scene::shader::{DrawShader, ShaderKey};

pub(crate) struct Programs<'c> {
    pub(crate) device: &'c wgpu::Device,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) composites: &'c mut Vec<LayerCompositePipeline>,
    pub(crate) shaders: &'c mut ShaderPrograms,
}

impl<'c> Programs<'c> {
    pub(crate) const fn of(ctx: &'c mut WalkCtx<'_>) -> Self {
        Self {
            device: ctx.device,
            format: ctx.format,
            composites: &mut *ctx.layer_pipelines,
            shaders: &mut *ctx.shader_programs,
        }
    }

    pub(crate) fn ready(
        self,
        key: ShaderKey,
        form: Form,
    ) -> Option<(&'c QuadPipeline, &'c mut LayerCompositePipeline)> {
        let composite =
            LayerCompositePipeline::for_format(self.composites, self.device, self.format);
        let [uniform, texture] = composite.layouts();
        let groups = match form {
            Form::Fill => [Some(uniform), None],
            Form::Layer => [Some(uniform), Some(texture)],
        };
        let used = groups.iter().take_while(|group| group.is_some()).count();
        let quad = self.shaders.ready(
            self.device,
            (key, form, self.format),
            groups.get(..used).unwrap_or_default(),
        )?;
        Some((quad, composite))
    }
}

impl SceneWalk<'_> {
    pub(super) fn queue_shader(&mut self, draw: &DrawShader) {
        let transform = self.state.current_transform();
        if self
            .state
            .clip_rect(transform_rect(transform, draw.rect))
            .is_none()
        {
            return;
        }
        let key = draw.shading.key;
        if Programs::of(&mut self.ctx).ready(key, Form::Fill).is_none() {
            self.flush_shaders();
            self.queue_rect(&draw.fallback());
            return;
        }
        if self.ctx.pending.shader_key != Some(key) {
            self.flush_shaders();
            self.ctx.pending.shader_key = Some(key);
        }
        let place = Place {
            rect: draw.rect,
            radii: draw.radii,
            opacity: crate::scene::command::opacity::from_basis(draw.opacity)
                * self.state.current_opacity(),
            transform,
            layer_uv: [0.0; 2],
        };
        let space = BlendSpace::from_format(self.ctx.format);
        let instance = ShaderInstance::new(&place, &draw.shading, space, self.ctx.scale);
        self.ctx.pending.shaders.push(instance);
    }

    pub(super) fn flush_shaders(&mut self) {
        let Some(key) = self.ctx.pending.shader_key.take() else {
            return;
        };
        if self.ctx.pending.shaders.is_empty() {
            return;
        }
        let clip = self.state.current_clip();
        let ctx = &mut self.ctx;
        let drawn = u64::try_from(ctx.pending.shaders.len()).unwrap_or(u64::MAX);
        count(Counter::GpuShaderDraws, drawn);
        let instances = FrameUploads::upload(ctx.encoder, &ctx.pending.shaders, &mut *ctx.stats);
        ctx.pending.shaders.clear();
        let Some(instances) = instances else {
            return;
        };
        let paint = PaintCtx {
            device: ctx.device,
            queue: ctx.queue,
            encoder: &mut *ctx.encoder,
            view: ctx.view,
            viewport: ctx.viewport,
            clip,
            scale: ctx.scale,
        };
        let programs = Programs {
            device: ctx.device,
            format: ctx.format,
            composites: &mut *ctx.layer_pipelines,
            shaders: &mut *ctx.shader_programs,
        };
        if let Some((quad, composite)) = programs.ready(key, Form::Fill) {
            composite.paint_shader(paint, quad, &instances, None);
        }
    }
}
