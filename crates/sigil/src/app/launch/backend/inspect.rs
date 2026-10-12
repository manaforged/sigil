use super::{Backend, Surface};
use crate::renderer::capture::ImageTarget;

pub(in crate::app::launch) fn inspect_shot(
    app: &mut crate::app::embed::Host,
    renderer: &mut Backend,
    surface: Option<&Surface>,
    viewport: (f32, f32),
) -> Result<serde_json::Value, String> {
    let Some((cx, _, root)) = app.tree() else {
        return Err("no mounted tree".into());
    };
    let scene =
        super::super::scene_cache::window_scene(cx, root, viewport, !renderer.is_transparent());
    let size = match surface {
        Some(Surface::Wgpu(surface)) => surface.size(),
        Some(Surface::Cpu(surface)) => surface.size(),
        None => return Err("window surface is not initialized yet".into()),
    };
    let clear = renderer.clear_color();
    let image = renderer
        .render_image(
            &scene,
            ImageTarget {
                size,
                viewport,
                clear,
            },
        )
        .map_err(|err| err.to_string())?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|err| format!("software renderer failed: {err}"))?;
    Ok(serde_json::json!({
        "png": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png),
        "width": size.width,
        "height": size.height,
    }))
}
