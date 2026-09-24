#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil::widgets::mermaid;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        drop(mermaid(s));
    }
});
