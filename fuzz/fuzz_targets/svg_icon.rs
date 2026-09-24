#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil::icons::IconBaker;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        drop(IconBaker::new().bake(s, 24, 24));
    }
});
