#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil::node::Cx;
use sigil::reactive::{Owner, signal};
use sigil::widgets::html;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        Owner::scope("fuzz-html", || {
            let mut cx = Cx::new_root();
            let _root = cx.add(html(signal(s.to_owned())));
            cx.flush_reactive_children();
        });
    }
});
