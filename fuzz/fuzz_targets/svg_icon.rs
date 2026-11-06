#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil::widgets::icon_svg;
use sigil_kittest::Harness;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let svg = s.to_owned();
        let mut ui = Harness::new(move || icon_svg(svg.clone()));
        ui.settle();
    }
});
