use crate::kernel::Cx;
use crate::reactive::{Runtime, signal};

use super::transport;

#[test]
fn transport_builds() {
    Runtime::scope("transport-builds", || {
        let playing = signal(false);
        let progress = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(transport(playing, progress, || {}));
        let node = cx.get(id).expect("transport root");
        assert!(
            !node.children().is_empty(),
            "the transport shows its controls"
        );
    });
}

#[test]
fn transport_seek_builds() {
    Runtime::scope("transport-seek", || {
        let playing = signal(false);
        let progress = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let id = cx.add(transport(playing, progress, || {}).seek(|_| {}));
        let node = cx.get(id).expect("transport seek root");
        assert!(!node.children().is_empty(), "the seek slider is present");
    });
}
