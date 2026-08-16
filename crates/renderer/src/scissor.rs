//! Logical clip rectangles converted to physical GPU scissors.

use sigil_hal::Rect;

/// Apply `clip` to `pass`. Returns `false` when a requested clip has no
/// physical pixels, allowing the caller to skip the draw.
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
    clippy::cast_sign_loss
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
mod tests {
    use sigil_hal::{Point, Rect};

    use super::physical_rect;

    #[test]
    fn converts_fractional_logical_clip_to_physical_pixels() {
        let clip = Rect::new(Point::new(2.25, 3.75), 4.5, 5.0);
        assert_eq!(physical_rect(clip, 2.0, (20.0, 20.0)), Some([4, 7, 10, 11]));
    }

    #[test]
    fn clamps_to_the_framebuffer_and_rejects_empty_regions() {
        let partial = Rect::new(Point::new(-5.0, 8.0), 10.0, 10.0);
        assert_eq!(
            physical_rect(partial, 1.0, (12.0, 12.0)),
            Some([0, 8, 5, 4])
        );

        let outside = Rect::new(Point::new(20.0, 20.0), 4.0, 4.0);
        assert_eq!(physical_rect(outside, 1.0, (12.0, 12.0)), None);
    }
}
