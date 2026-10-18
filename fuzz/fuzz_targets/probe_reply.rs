#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sigil::fuzzing::probe_reply(&String::from_utf8_lossy(data));
});
