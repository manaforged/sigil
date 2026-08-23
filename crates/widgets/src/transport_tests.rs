use sigil_kernel::Cx;
use sigil_reactive::{Runtime, signal};

use super::transport;

#[test]
fn transport_builds() {
    Runtime::scope("transport-builds", || {
        let playing = signal(false);
        let progress = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(transport(playing, progress, || {}));
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn transport_seek_builds() {
    Runtime::scope("transport-seek", || {
        let playing = signal(false);
        let progress = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(transport(playing, progress, || {}).seek(|_| {}));
        assert!(cx.get(id).is_some());
    });
}
