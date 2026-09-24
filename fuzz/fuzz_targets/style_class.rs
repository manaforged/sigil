#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil::style::{parse_class_string, parse_stateful_token};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        drop(parse_class_string(s));
        drop(parse_stateful_token(s));
    }
});
