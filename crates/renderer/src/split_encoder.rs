//! Command-encoder wrapper that splits pathological submissions.
//!
//! Apple-silicon Metal kills the device — "Device is lost", reason Unknown,
//! no validation error — once one command buffer accumulates roughly 2,000
//! render passes. A tall unvirtualized scene crosses that line with one pass
//! per glyph run plus interleaved row fills (PORT.md gap 23); the loss only
//! surfaced later, when the capture readback mapped against the dead device.
//! [`SplitEncoder`] counts passes and, past [`MAX_PASSES_PER_SUBMISSION`],
//! submits the encoder it owns and starts a fresh one. Queue submissions run
//! in FIFO order and every pass is load/store on the same attachments, so
//! chunking is visually identical. Normal frames never cross the budget and
//! keep DR-054's single submission.

use std::mem::replace;

/// Render passes recorded into one encoder before it is submitted and
/// swapped for a fresh one. Comfortably below the observed ~2,050-pass
/// Metal device-loss threshold; the swap costs one extra submission.
pub(crate) const MAX_PASSES_PER_SUBMISSION: u32 = 1_024;

/// Pass-counting owner of the frame's command encoder. See module docs.
///
/// Owns `device` / `queue` (cheap Arc clones) so the wrapper can submit
/// intermediate chunks itself; the caller gets the live encoder back from
/// [`into_inner`](Self::into_inner) and submits the final chunk. Passes the
/// caller recorded before the wrap (the initial clear) are not counted.
pub(crate) struct SplitEncoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    passes: u32,
}

impl SplitEncoder {
    /// Wrap `encoder`, cloning `device` / `queue` handles for chunk submits.
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

    /// Begin `desc`, first splitting the submission when the owned encoder
    /// has reached the pass budget.
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

    /// Hand back the live encoder so the owner can finish + submit the
    /// final chunk.
    pub(crate) fn into_inner(self) -> wgpu::CommandEncoder {
        self.encoder
    }
}
