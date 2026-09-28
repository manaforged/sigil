#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil_kittest::parity::check_cpu;
use sigil_kittest::scene_gen::{Entropy, generate};

fuzz_target!(|data: &[u8]| {
    let scene = generate(&mut Entropy::from_bytes(data));
    if let Err(message) = check_cpu(&scene) {
        panic!(
            "{message}: {:?} x{} {:#?}",
            scene.viewport, scene.scale, scene.commands
        );
    }
});
