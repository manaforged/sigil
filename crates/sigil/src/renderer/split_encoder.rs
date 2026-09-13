use std::mem::replace;

pub(crate) const MAX_PASSES_PER_SUBMISSION: u32 = 1_024;

pub(crate) struct SplitEncoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    passes: u32,
}

impl SplitEncoder {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: wgpu::CommandEncoder,
    ) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),
            encoder,
            passes: 0,
        }
    }

    pub(crate) fn begin_render_pass(
        &mut self,
        desc: &wgpu::RenderPassDescriptor<'_>,
    ) -> wgpu::RenderPass<'_> {
        self.passes += 1;
        if self.passes > MAX_PASSES_PER_SUBMISSION {
            let fresh = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sigil-renderer split submission"),
                });
            let full = replace(&mut self.encoder, fresh);
            self.queue.submit(core::iter::once(full.finish()));
            self.passes = 1;
        }
        self.encoder.begin_render_pass(desc)
    }

    pub(crate) fn into_inner(self) -> wgpu::CommandEncoder {
        self.encoder
    }
}
