use super::DecorationPipeline;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn decoration_pipeline_constructs_against_real_device() {
    let Some(device) = crate::renderer::test_state::shared_test_device() else {
        eprintln!("no device - skipping the pipeline shader-validation test");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let pipeline = DecorationPipeline::new(device, format);
    assert_eq!(pipeline.format(), format);
}
