use crate::app::WindowCommand;
use crate::app::error::AppResult;
use crate::app::window::{WindowId, take_open_requests};
use crate::hal::Shell as _;
use crate::shell::{WinitShell, WinitWindowHandle};

use super::create_initial_window;
use super::logical_size_from_config;
use super::record::{CloseOutcome, Routed, WindowRecord, Windows};

pub(super) fn open_requested(shell: &mut WinitShell, windows: &mut Windows) -> AppResult<()> {
    for request in take_open_requests() {
        let logical = logical_size_from_config(request.config.size);
        let handle = create_initial_window(shell, &request.config, logical)?;
        windows.insert(WindowRecord::new(
            request.id,
            handle,
            request.config,
            request.mount,
        )?);
        shell.request_redraws();
    }
    Ok(())
}

pub(super) fn dispatch_events(shell: &mut WinitShell, windows: &mut Windows) {
    for (handle, event) in shell.drain_tagged_events() {
        match windows.route(handle, event) {
            Routed::Close => apply_close(shell, windows, handle),
            Routed::Destroyed => {
                windows.remove(handle);
                shell.remove_window(handle);
            }
            Routed::Delivered | Routed::Dropped => {}
        }
    }
}

fn apply_close(shell: &mut WinitShell, windows: &mut Windows, handle: WinitWindowHandle) {
    match windows.close_requested(handle) {
        CloseOutcome::Removed { last } => {
            shell.remove_window(handle);
            if last {
                shell.request_exit();
            }
        }
        CloseOutcome::Hidden => {
            if let Some(window) = shell.window_for(handle) {
                window.set_visible(false);
            }
        }
        CloseOutcome::Kept => {}
    }
}

pub(super) fn apply_commands(
    shell: &mut WinitShell,
    windows: &mut Windows,
    default: Option<WinitWindowHandle>,
) {
    for (target, command) in crate::events::take_window_commands() {
        let Some(handle) = resolve_target(windows, target, default) else {
            continue;
        };
        note_command(windows, handle, &command);
        if let Err(err) = shell.window_command(&handle, command) {
            tracing::warn!(target: "sigil::shell", error = %err, "window command failed");
        }
    }
}

fn resolve_target(
    windows: &Windows,
    target: Option<WindowId>,
    default: Option<WinitWindowHandle>,
) -> Option<WinitWindowHandle> {
    target.map_or_else(
        || default.or_else(|| windows.first_handle()),
        |id| windows.handle_of(id),
    )
}

fn note_command(windows: &mut Windows, handle: WinitWindowHandle, command: &WindowCommand) {
    match command {
        WindowCommand::SetZoom(percent) if *percent > 0 => {
            if let Some(record) = windows.get_mut(handle) {
                record.view.config.user_zoom = f32::from(*percent) / 100.0;
            }
        }
        WindowCommand::Show | WindowCommand::Focus => windows.set_hidden(handle, false),
        WindowCommand::Hide => windows.set_hidden(handle, true),
        _ => {}
    }
}
