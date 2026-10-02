#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]
use crate::profile::{count_add, enabled};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FrameStats {
    pub scene_commands: usize,
    pub rectangle_batches: usize,
    pub rectangle_instances: usize,
    pub fill_batches: usize,
    pub fill_instances: usize,
    pub shadow_batches: usize,
    pub shadow_instances: usize,
    pub decoration_batches: usize,
    pub decoration_instances: usize,
    pub glyph_runs: usize,
    pub text_draws: usize,
    pub sprite_draws: usize,
    pub offscreen_layers: usize,
    pub offscreen_pixels: u64,
    pub frame_upload_buffers: usize,
    pub frame_upload_capacity_bytes: u64,
    pub frame_upload_writes: usize,
    pub frame_uploaded_bytes: u64,
    pub redrawn_pixels: u64,
}

impl FrameStats {
    #[must_use]
    pub const fn batches(self) -> u64 {
        (self.rectangle_batches
            + self.fill_batches
            + self.shadow_batches
            + self.decoration_batches
            + self.text_draws
            + self.sprite_draws) as u64
    }

    pub fn record_profile(self) {
        if !enabled() {
            return;
        }
        for (name, value) in [
            (
                crate::profile::Counter::SceneCmds,
                self.scene_commands as u64,
            ),
            (
                crate::profile::Counter::GpuRects,
                self.rectangle_instances as u64,
            ),
            (crate::profile::Counter::GpuGlyphs, self.glyph_runs as u64),
            (
                crate::profile::Counter::GpuSprites,
                self.sprite_draws as u64,
            ),
            (
                crate::profile::Counter::GpuLayers,
                self.offscreen_layers as u64,
            ),
            (
                crate::profile::Counter::GpuLayerPixels,
                self.offscreen_pixels,
            ),
            (
                crate::profile::Counter::GpuUploadBuffers,
                self.frame_upload_buffers as u64,
            ),
            (
                crate::profile::Counter::GpuUploadCapacityBytes,
                self.frame_upload_capacity_bytes,
            ),
            (
                crate::profile::Counter::GpuUploadWrites,
                self.frame_upload_writes as u64,
            ),
            (
                crate::profile::Counter::GpuUploadBytes,
                self.frame_uploaded_bytes,
            ),
            (
                crate::profile::Counter::GpuRedrawnPixels,
                self.redrawn_pixels,
            ),
            (crate::profile::Counter::RenderFrames, 1),
            (crate::profile::Counter::GpuBatches, self.batches()),
        ] {
            count_add(name, value);
        }
    }

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
