use super::*;
use crate::renderer::capture::ImageTarget;
use crate::scene::SceneBuilder;

const TARGET: ImageTarget = ImageTarget {
    size: PhysicalSize {
        width: 8,
        height: 4,
    },
    viewport: (8.0, 4.0),
    clear: Color::BLACK,
};

fn backend_without_adapter(fallback: bool) -> Backend {
    let mut backend = Backend::from_env();
    backend.inner = BackendInner::Wgpu(Box::new(WgpuRenderer::without_adapter()));
    backend.fallback = fallback;
    backend
}

#[test]
fn render_image_falls_back_to_cpu_when_no_adapter_exists() {
    let mut backend = backend_without_adapter(true);
    let image = backend
        .render_image(&SceneBuilder::new().finish(), TARGET)
        .expect("cpu fallback image");
    assert_eq!(image.dimensions(), (8, 4));
    assert!(matches!(backend.inner, BackendInner::Cpu(_)));
    assert_eq!(
        backend.status(),
        BackendStatus {
            kind: profile::Backend::Cpu,
            fallback: FallbackCause::NoAdapter,
        }
    );
}

#[test]
fn render_image_returns_the_gpu_error_when_fallback_is_off() {
    let mut backend = backend_without_adapter(false);
    let error = backend
        .render_image(&SceneBuilder::new().finish(), TARGET)
        .expect_err("no adapter");
    assert_eq!(error.kind(), ErrorKind::NoAdapter);
    assert!(matches!(backend.inner, BackendInner::Wgpu(_)));
}
