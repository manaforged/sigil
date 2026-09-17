use crate::profile::{enabled, gauge};

pub(crate) fn snapshot(instance: &wgpu::Instance) {
    if !enabled() {
        return;
    }
    let Some(report) = instance.generate_report() else {
        return;
    };
    let hub = report.hub;
    for (name, value) in [
        ("wgpu.ids.buffers", hub.buffers.num_allocated),
        ("wgpu.ids.textures", hub.textures.num_allocated),
        ("wgpu.ids.texture_views", hub.texture_views.num_allocated),
        ("wgpu.ids.bind_groups", hub.bind_groups.num_allocated),
        (
            "wgpu.ids.command_buffers",
            hub.command_buffers.num_allocated,
        ),
        (
            "wgpu.ids.render_pipelines",
            hub.render_pipelines.num_allocated,
        ),
    ] {
        gauge(name, value as u64);
    }
}
