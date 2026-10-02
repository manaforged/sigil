use super::setup::{SurfaceParams, UNIT_QUAD_VERTEX_COUNT};
use super::{
    RasterFrame, RasterGroup, RasterInstance, RasterTextPipeline, normalize_uvs, scissor_rect,
};
use crate::renderer::split_encoder::SplitEncoder;

impl RasterTextPipeline {
    pub(crate) fn draw_batch(
        &mut self,
        encoder: &mut SplitEncoder,
        frame: &RasterFrame<'_>,
        instances: &mut [RasterInstance],
        groups: &[RasterGroup],
    ) {
        if instances.is_empty() {
            return;
        }
        normalize_uvs(instances, self.atlas.size());
        let (uniform, offset, _) = encoder.uniform(&SurfaceParams {
            surface_size: [frame.surface_width.max(1.0), frame.surface_height.max(1.0)],
            _pad: [0.0, 0.0],
        });
        self.prepare_uniforms(frame.device, uniform);
        let Some((_, _, bind)) = self.uniforms.as_ref() else {
            return;
        };
        let uploaded = encoder.upload(instances);
        let size = frame.view.texture().size();
        let attachment = [Some(wgpu::RenderPassColorAttachment {
            view: frame.view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })];
        let pass = encoder.begin_render_pass(
            "raster-text-pass",
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &attachment,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            },
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind, &[offset]);
        pass.set_vertex_buffer(0, self.quad_vb.slice(..));
        pass.set_vertex_buffer(1, uploaded.buffer.slice(uploaded.bytes));
        for group in groups {
            let Some((x, y, w, h)) = scissor_for(group, frame, size) else {
                continue;
            };
            pass.set_scissor_rect(x, y, w, h);
            pass.draw(0..UNIT_QUAD_VERTEX_COUNT, group.range.clone());
        }
        pass.set_scissor_rect(0, 0, size.width, size.height);
    }
}

fn scissor_for(
    group: &RasterGroup,
    frame: &RasterFrame<'_>,
    size: wgpu::Extent3d,
) -> Option<(u32, u32, u32, u32)> {
    group
        .clip
        .map_or(Some((0, 0, size.width, size.height)), |clip| {
            scissor_rect(clip, frame.scale, frame.surface_width, frame.surface_height)
        })
}
