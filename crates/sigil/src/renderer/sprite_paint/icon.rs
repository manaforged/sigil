use crate::icons::{BakedIcon, IconBaker, builtin};

use super::{AtlasDesc, SpritePipeline};
use crate::renderer::sprite_atlas::builtin_icon_index;

pub(super) fn is_builtin_icon_atlas(raw_id: u32) -> bool {
    builtin_icon_index(raw_id).is_some()
}

impl SpritePipeline {
    pub(crate) fn register_icon(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        icon: &BakedIcon,
    ) -> u32 {
        let rgba: Vec<u8> = icon
            .alpha
            .iter()
            .flat_map(|&a| [255_u8, 255, 255, a])
            .collect();
        self.register_atlas(
            device,
            queue,
            &AtlasDesc {
                bytes: &rgba,
                width: icon.width,
                height: icon.height,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
            },
        )
    }

    pub(super) fn resolve_builtin_icon_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw_id: u32,
    ) -> Option<usize> {
        let index = builtin_icon_index(raw_id)?;
        if let Some(slot) = self.builtin_icon_slots.get(index).copied().flatten() {
            return Some(slot);
        }
        let (_, svg) = builtin::ALL.get(index)?;
        let icon = IconBaker::new().bake(svg, 128, 128).ok()?;
        if self.register_icon(device, queue, &icon) == u32::MAX {
            return None;
        }
        let slot = self.atlases.len().checked_sub(1)?;
        if let Some(icon_slot) = self.builtin_icon_slots.get_mut(index) {
            *icon_slot = Some(slot);
        }
        Some(slot)
    }
}
