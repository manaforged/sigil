use crate::renderer::blur_paint::BlurPipeline;

pub(super) fn blur_pipeline_for<'a>(
    pipelines: &'a mut Vec<BlurPipeline>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> &'a mut BlurPipeline {
    let idx = pipelines
        .iter()
        .position(|p| p.format() == format)
        .unwrap_or_else(|| {
            pipelines.push(BlurPipeline::new(device, format));
            pipelines.len() - 1
        });
    pipelines
        .get_mut(idx)
        .expect("blur pipeline index was just constructed from the vec")
}
