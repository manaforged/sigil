#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil::kernel::Cx;
use sigil::reactive::{Runtime, signal};
use sigil::widgets::markdown;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        Runtime::scope("fuzz-markdown", || {
            let mut cx = Cx::new_root();
            let _root = cx.add(markdown(signal(s.to_owned())));
            cx.flush_reactive_children();
        });
    }
});
