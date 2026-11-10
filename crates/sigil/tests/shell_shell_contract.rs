#![cfg(feature = "kittest")]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::shell::{WinitShell, WinitWaker};

#[test]
fn waker_is_send_sync_clone_static() {
    fn assert_bounds<T: Send + Sync + Clone + 'static>() {}
    assert_bounds::<<WinitShell as sigil::hal::Shell>::Waker>();
    assert_bounds::<WinitWaker>();
}

#[test]
fn shell_error_implements_std_error() {
    fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
    assert_error::<sigil::Error>();
}

#[test]
fn take_failed_windows_signature_is_owned_vec() {
    #[allow(dead_code)]
    fn probe(shell: &mut WinitShell) -> Vec<(sigil::shell::WinitWindowHandle, String)> {
        shell.take_failed_windows()
    }
}
