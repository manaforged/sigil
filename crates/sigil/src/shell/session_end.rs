#![expect(
    unsafe_code,
    reason = "Windows reports session end only to a window procedure, so the shell subclasses its windows"
)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Shutdown::{ShutdownBlockReasonCreate, ShutdownBlockReasonDestroy};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WM_ENDSESSION, WM_NCDESTROY, WM_QUERYENDSESSION,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_string;

type Wake = Box<dyn Fn()>;

const SUBCLASS_ID: usize = 1;

pub(crate) fn watch(window: &winit::window::Window, wake: Wake) {
    let Some(hwnd) = hwnd(window) else {
        tracing::warn!(target: "sigil::shell", "window has no Win32 handle; session end is not watched");
        return;
    };
    let data = Box::into_raw(Box::new(wake)) as usize;
    // SAFETY: `hwnd` is the live window winit just created on this thread, and `data` stays owned by the subclass until WM_NCDESTROY frees it.
    let installed = unsafe { SetWindowSubclass(hwnd, Some(session_proc), SUBCLASS_ID, data) };
    if installed == 0 {
        // SAFETY: the subclass was not installed, so nothing else holds `data`.
        drop(unsafe { Box::from_raw(data as *mut Wake) });
        tracing::warn!(target: "sigil::shell", "session end subclass install failed");
    }
}

fn hwnd(window: &winit::window::Window) -> Option<HWND> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get() as HWND),
        _ => None,
    }
}

unsafe extern "system" fn session_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    match msg {
        WM_QUERYENDSESSION if super::exit_hook::pending() => block_shutdown(hwnd),
        WM_ENDSESSION => end_session(hwnd, wparam != 0, data),
        WM_NCDESTROY => release(hwnd, id, data),
        _ => {}
    }
    // SAFETY: Windows calls this procedure with the arguments of a live message for `hwnd`, which go on unchanged to the next procedure in the chain.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn block_shutdown(hwnd: HWND) {
    let reason: Vec<u16> = theme_string(StringKey::SavingOnExit)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `hwnd` is the window receiving this message and `reason` is a NUL-terminated UTF-16 string that outlives the call.
    if unsafe { ShutdownBlockReasonCreate(hwnd, reason.as_ptr()) } == 0 {
        tracing::warn!(target: "sigil::shell", "shutdown block reason was not registered");
    }
}

fn end_session(hwnd: HWND, ending: bool, data: usize) {
    if ending {
        if catch_unwind(AssertUnwindSafe(super::exit_hook::run)).is_err() {
            tracing::error!(target: "sigil::shell", "exit hook panicked during session end");
        }
        crate::app::exit::request_exit();
        // SAFETY: `data` is the wake closure `watch` boxed for this subclass; WM_NCDESTROY has not freed it while the window still receives messages.
        let wake = unsafe { &*(data as *const Wake) };
        wake();
    }
    // SAFETY: `hwnd` is the window receiving this message; destroying a reason that was never created is a harmless failure.
    unsafe { ShutdownBlockReasonDestroy(hwnd) };
}

fn release(hwnd: HWND, id: usize, data: usize) {
    // SAFETY: the window is being destroyed; this removes the subclass `watch` installed with the same procedure and id.
    unsafe { RemoveWindowSubclass(hwnd, Some(session_proc), id) };
    // SAFETY: `data` came from `Box::into_raw` in `watch`, and this is the last message the window receives.
    drop(unsafe { Box::from_raw(data as *mut Wake) });
}
