use crate::profile::{Counter, enabled, gauge};

pub(crate) fn snapshot(instance: &wgpu::Instance) {
    if !enabled() {
        return;
    }
    let Some(report) = instance.generate_report() else {
        return;
    };
    let hub = report.hub;
    for (name, value) in [
        (Counter::WgpuBuffers, hub.buffers.num_allocated),
        (Counter::WgpuTextures, hub.textures.num_allocated),
        (Counter::WgpuTextureViews, hub.texture_views.num_allocated),
        (Counter::WgpuBindGroups, hub.bind_groups.num_allocated),
        (
            Counter::WgpuCommandBuffers,
            hub.command_buffers.num_allocated,
        ),
        (
            Counter::WgpuRenderPipelines,
            hub.render_pipelines.num_allocated,
        ),
    ] {
        gauge(name, value as u64);
    }
}
