use pollster::FutureExt as _;

use super::ScenePipeline;

/// Constructs the scene pipeline against a real GPU adapter so the
/// shader is parsed + validated. Catches WGSL keyword collisions
/// (the kind of bug that crashed `hello-style` on launch with
/// `name 'meta' is a reserved keyword`).
#[test]
fn scene_pipeline_constructs_against_real_device() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .block_on()
    else {
        eprintln!("no adapter — skipping scene-pipeline shader-validation test");
        return;
    };
    let Ok((device, _queue)) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("scene-paint-test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
    else {
        eprintln!("no device — skipping scene-pipeline shader-validation test");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let pipeline = ScenePipeline::new(&device, format);
    assert_eq!(pipeline.format(), format);
}
