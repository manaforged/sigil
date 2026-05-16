use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;

use sigil_hal::{Color, PhysicalSize, Renderer, Shell};
use sigil_renderer::{
    CommandScene, ScreenSubmission, WgpuRenderer, WgpuSurface, into_window_handle,
};
use sigil_shell::WinitShell;

#[allow(unreachable_pub)]
#[allow(dead_code)]
pub fn run(
    title: &str,
    scene: CommandScene,
    viewport: (f32, f32),
    clear: Color,
) -> Result<(), Box<dyn Error>> {
    let mut shell = WinitShell::new()?;
    let mut renderer = WgpuRenderer::new()?;
    let window = shell.create_window(title, viewport_size(viewport))?;
    let surface_slot: Rc<RefCell<Option<WgpuSurface>>> = Rc::new(RefCell::new(None));

    shell.run(move |shell| {
        let mut slot = surface_slot.borrow_mut();
        if slot.is_none()
            && let Some(window) = shell.window_for(window)
        {
            let size = window.inner_size();
            let handle = into_window_handle(Arc::clone(window));
            if let Ok(created) = renderer.create_surface(
                handle,
                PhysicalSize::new(size.width.max(1), size.height.max(1)),
            ) {
                *slot = Some(created);
            }
        }
        if let Some(target_surface) = slot.as_mut() {
            let _ = renderer.render_command_scene(target_surface, &scene, clear, viewport);
        }
    })?;
    Ok(())
}

#[allow(unreachable_pub)]
#[allow(dead_code)]
pub fn run_split(
    title: &str,
    scenes: [CommandScene; 2],
    viewport: (f32, f32),
    clear: Color,
) -> Result<(), Box<dyn Error>> {
    let mut shell = WinitShell::new()?;
    let mut presenter = WgpuRenderer::new()?;
    let mut screen_renderer = WgpuRenderer::new()?;
    let window = shell.create_window(title, viewport_size(viewport))?;
    let surface_slot: Rc<RefCell<Option<WgpuSurface>>> = Rc::new(RefCell::new(None));

    shell.run(move |shell| {
        let mut slot = surface_slot.borrow_mut();
        if slot.is_none()
            && let Some(window) = shell.window_for(window)
        {
            let size = window.inner_size();
            let handle = into_window_handle(Arc::clone(window));
            if let Ok(created) = presenter.create_surface(
                handle,
                PhysicalSize::new(size.width.max(1), size.height.max(1)),
            ) {
                *slot = Some(created);
            }
        }
        if let Some(target_surface) = slot.as_mut() {
            let format = target_surface.format();
            let _ = presenter.render_frame(target_surface, clear, |device, queue, view| {
                let _ = screen_renderer.submit_screen(
                    &ScreenSubmission {
                        device,
                        queue,
                        view,
                        format,
                        viewport,
                        clear: Some(clear),
                    },
                    &scenes[0],
                );
                let _ = screen_renderer.submit_screen(
                    &ScreenSubmission {
                        device,
                        queue,
                        view,
                        format,
                        viewport,
                        clear: None,
                    },
                    &scenes[1],
                );
            });
        }
    })?;
    Ok(())
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[allow(clippy::missing_const_for_fn)]
fn viewport_size(viewport: (f32, f32)) -> PhysicalSize {
    PhysicalSize::new(
        viewport.0.max(1.0).ceil() as u32,
        viewport.1.max(1.0).ceil() as u32,
    )
}
