//! Renders each showcase to a PNG in the directory given as the first argument.

use std::error::Error;
use std::path::Path;

use sigil::prelude::*;
use sigil_kittest::Harness;
use sigil_showcase::{SIZE, almanac, darkroom, deck, focus};

const SCALE: f32 = 2.0;
const TURNS: [f32; 3] = [0.0, 0.3, 1.0];
const GRADE: [f32; 4] = [1.1, 1.25, 0.6, 0.55];

fn main() -> Result<(), Box<dyn Error>> {
    let dir = std::env::args().nth(1).ok_or("usage: shots <directory>")?;
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir)?;
    shot(dir, "almanac-noon", || almanac::at(12.0))?;
    shot(dir, "almanac-golden", || almanac::at(16.75))?;
    shot(dir, "almanac-night", || almanac::at(22.0))?;
    shot(dir, "darkroom-rest", darkroom::app)?;
    shot(dir, "darkroom-graded", || darkroom::graded(&GRADE))?;
    shot(dir, "deck", || deck::turned(TURNS))?;
    shot(dir, "focus", focus::app)?;
    shot(dir, "focus-clear", || focus::frosted(0.0))?;
    let cpu = Harness::new(focus::app)
        .pixels(SIZE[0], SIZE[1])
        .image()?
        .ok_or("no CPU image")?;
    cpu.save(dir.join("focus-cpu.png"))?;
    Ok(())
}

fn shot<N: IntoNode>(dir: &Path, name: &str, app: impl Fn() -> N) -> Result<(), Box<dyn Error>> {
    let image = Harness::render(app)
        .size(SIZE)
        .scale(SCALE)
        .image()?
        .ok_or("no GPU adapter")?;
    image.save(dir.join(format!("{name}.png")))?;
    Ok(())
}
