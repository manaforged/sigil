use pollster::FutureExt as _;

use super::ShadowPipeline;
use super::shadow_shader::SHADOW_SHADER;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn shadow_pipeline_constructs_against_real_device() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .block_on()
    else {
        eprintln!("no adapter - skipping shadow-pipeline shader-validation test");
        return;
    };
    let Ok((device, _queue)) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("shadow-paint-test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
    else {
        eprintln!("no device - skipping shadow-pipeline shader-validation test");
        return;
    };
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let pipeline = ShadowPipeline::new(&device, format);
    assert_eq!(pipeline.format(), format);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn inner_shadow_shader_clips_and_inverts_coverage() {
    assert!(SHADOW_SHADER.contains("if shadow_kind >= 0.5"));
    assert!(SHADOW_SHADER.contains("if d_surface > 0.0"));
    assert!(SHADOW_SHADER.contains("let inner_spread = -spread"));
    assert!(SHADOW_SHADER.contains("coverage = 1.0 - shadow_coverage"));
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn glow_shader_branch_present_and_ordered_before_inner() {
    assert!(SHADOW_SHADER.contains("if shadow_kind >= 1.5"));
    let glow_pos = SHADOW_SHADER
        .find("if shadow_kind >= 1.5")
        .expect("glow branch");
    let inner_pos = SHADOW_SHADER
        .find("} else if shadow_kind >= 0.5")
        .expect("inner branch");
    assert!(
        glow_pos < inner_pos,
        "glow branch must precede inner branch"
    );
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn glow_kind_as_f32_is_two() {
    assert!((super::ShadowKind::Glow.as_f32() - 2.0).abs() < f32::EPSILON);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn shadow_glow_constructor_sets_kind_and_zero_offset() {
    use sigil_style::Rgba8;
    let g = super::Shadow::glow(
        Rgba8 {
            r: 80,
            g: 200,
            b: 255,
            a: 128,
        },
        8.0,
        18.0,
    );
    assert_eq!(g.kind, super::ShadowKind::Glow);
    assert!((g.offset[0]).abs() < f32::EPSILON);
    assert!((g.offset[1]).abs() < f32::EPSILON);
    assert!((g.blur - 18.0).abs() < 1e-5);
    assert!((g.spread - 8.0).abs() < 1e-5);
    assert!((g.color[3] - 128.0 / 255.0).abs() < 1e-4);
}
