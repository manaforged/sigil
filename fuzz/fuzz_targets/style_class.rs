#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = sigil::fuzzing::parse_class_string(s);
        let _ = sigil::fuzzing::parse_stateful_token(s);
    }
});
