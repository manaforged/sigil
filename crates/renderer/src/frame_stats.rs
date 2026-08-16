//! Exact counters from the most recently compiled command scene.

/// Scene/compiler work recorded for one command-scene render.
///
/// These counters describe work submitted by Sigil's command walker. They are
/// not GPU timestamps: use backend timestamp queries or an external profiler
/// when elapsed GPU time is required.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FrameStats {
    /// Commands in the source [`CommandScene`](crate::CommandScene).
    pub scene_commands: usize,
    /// Consecutive rectangle batches submitted to the rectangle pipeline.
    pub rectangle_batches: usize,
    /// Rectangle instances across all rectangle batches.
    pub rectangle_instances: usize,
    /// Consecutive gradient/pattern batches submitted to the fill pipeline.
    pub fill_batches: usize,
    /// Gradient/pattern instances across all fill batches.
    pub fill_instances: usize,
    /// Consecutive shadow batches submitted to the shadow pipeline.
    pub shadow_batches: usize,
    /// Shadow instances across all shadow batches.
    pub shadow_instances: usize,
    /// Consecutive decoration batches submitted to the decoration pipeline.
    pub decoration_batches: usize,
    /// Decoration instances across all decoration batches.
    pub decoration_instances: usize,
    /// Visible glyph-run commands handed to the text pipelines.
    pub glyph_runs: usize,
    /// Visible sprite/image draws handed to the sprite pipeline.
    pub sprite_draws: usize,
    /// Offscreen layer scopes rendered for compositing or effects.
    pub offscreen_layers: usize,
    /// Physical texture pixels requested by offscreen layer scopes.
    ///
    /// This includes device-scale expansion but not pool bucket padding.
    pub offscreen_pixels: u64,
    /// Transient GPU buffers allocated for primitive instances, gradient
    /// stops, and their shared viewport uniforms.
    pub frame_upload_buffers: usize,
    /// Bytes reserved across [`Self::frame_upload_buffers`].
    pub frame_upload_capacity_bytes: u64,
    /// Queue writes used to populate frame upload buffers.
    pub frame_upload_writes: usize,
    /// Bytes written into frame upload buffers.
    pub frame_uploaded_bytes: u64,
}

impl FrameStats {
    pub(crate) const fn record_rectangles(&mut self, instances: usize) {
        self.rectangle_batches += 1;
        self.rectangle_instances += instances;
    }

    pub(crate) const fn record_fills(&mut self, instances: usize) {
        self.fill_batches += 1;
        self.fill_instances += instances;
    }

    pub(crate) const fn record_shadows(&mut self, instances: usize) {
        self.shadow_batches += 1;
        self.shadow_instances += instances;
    }

    pub(crate) const fn record_decorations(&mut self, instances: usize) {
        self.decoration_batches += 1;
        self.decoration_instances += instances;
    }

    pub(crate) fn record_offscreen(&mut self, width: u32, height: u32) {
        self.offscreen_layers += 1;
        self.offscreen_pixels = self
            .offscreen_pixels
            .saturating_add(u64::from(width).saturating_mul(u64::from(height)));
    }

    pub(crate) const fn record_primitive_buffer(&mut self, bytes: u64) {
        self.frame_upload_buffers += 1;
        self.frame_upload_capacity_bytes = self.frame_upload_capacity_bytes.saturating_add(bytes);
    }

    pub(crate) const fn record_primitive_write(&mut self, bytes: u64) {
        self.frame_upload_writes += 1;
        self.frame_uploaded_bytes = self.frame_uploaded_bytes.saturating_add(bytes);
    }
}
