use sigil_hal::Rect;

pub(crate) fn apply(
    pass: &mut wgpu::RenderPass<'_>,
    clip: Option<Rect>,
    scale: f32,
    viewport: (f32, f32),
) -> bool {
    let Some(clip) = clip else {
        return true;
    };
    let Some([x, y, width, height]) = physical_rect(clip, scale, viewport) else {
        return false;
    };
    pass.set_scissor_rect(x, y, width, height);
    true
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn physical_rect(clip: Rect, scale: f32, viewport: (f32, f32)) -> Option<[u32; 4]> {
    let values = [
        clip.origin.x,
        clip.origin.y,
        clip.width,
        clip.height,
        scale,
        viewport.0,
        viewport.1,
    ];
    if values.iter().any(|value| !value.is_finite())
        || scale <= 0.0
        || viewport.0 <= 0.0
        || viewport.1 <= 0.0
    {
        return None;
    }

    let framebuffer_width = (viewport.0 * scale).clamp(0.0, u32::MAX as f32);
    let framebuffer_height = (viewport.1 * scale).clamp(0.0, u32::MAX as f32);
    let x0 = (clip.origin.x * scale)
        .floor()
        .clamp(0.0, framebuffer_width);
    let y0 = (clip.origin.y * scale)
        .floor()
        .clamp(0.0, framebuffer_height);
    let x1 = ((clip.origin.x + clip.width) * scale)
        .ceil()
        .clamp(0.0, framebuffer_width);
    let y1 = ((clip.origin.y + clip.height) * scale)
        .ceil()
        .clamp(0.0, framebuffer_height);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some([x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}

#[cfg(test)]
#[path = "scissor_tests.rs"]
mod tests;
