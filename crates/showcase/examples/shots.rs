//! Renders each showcase to a PNG in the directory given as the first argument.

use std::cell::Cell;
use std::error::Error;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use sigil::prelude::*;
use sigil_kittest::Harness;
use sigil_showcase::{SIZE, almanac, beam, darkroom, deck, focus, gallery, index, kit, tray};

const SCALE: f32 = 2.0;
const REVEAL_FRAMES: u32 = 4;
const REVEAL_STEP: f32 = 0.12;
const BEAM: f32 = 215.0;
const SCROLLED: f32 = 430.0;
const MID_ROLL: Duration = Duration::from_millis(90);
const MID_EXIT: Duration = Duration::from_millis(110);
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
    shot(dir, "gallery", gallery::app)?;
    shot(dir, "gallery-scrolled", || gallery::at(SCROLLED))?;
    staged(dir, "index", &index::fonts(), index::app)?;
    staged(dir, "index-first", &index::fonts(), || index::open(0))?;
    settled(dir, "kit", kit::app)?;
    settled(dir, "kit-sheet", || kit::sheet(true))?;
    shot(dir, "kit-gpu", kit::app)?;
    settled(dir, "tray", || tray::lifted(1.0))?;
    settled(dir, "tray-lowered", tray::app)?;
    settled(dir, "focus-cpu", focus::app)?;
    leaving(dir)?;
    settled(dir, "beam", || beam::lit(BEAM))?;
    rolling(dir)?;
    let strip = Harness::render(|| beam::lit(BEAM))
        .size(SIZE)
        .filmstrip(REVEAL_FRAMES, REVEAL_STEP)?
        .ok_or("no GPU adapter")?;
    strip.save(dir.join("beam-strip.png"))?;
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

fn leaving(dir: &Path) -> Result<(), Box<dyn Error>> {
    let open = Rc::new(Cell::new(None));
    let seen = Rc::clone(&open);
    let mut ui = Harness::new(move || {
        let rows = signal(vec![0, 1, 2, 3, 4]);
        seen.set(Some(rows));
        tray::listing(rows, 1.0)
    })
    .pixels(SIZE[0], SIZE[1]);
    ui.settle();
    let rows = open.get().ok_or("the tray did not mount")?;
    rows.update(|rows| rows.retain(|row| *row != 1));
    ui.step(MID_EXIT);
    let image = ui.image()?.ok_or("no CPU image")?;
    image.save(dir.join("tray-leaving.png"))?;
    Ok(())
}

fn settled<N: IntoNode + 'static>(
    dir: &Path,
    name: &str,
    app: impl Fn() -> N + 'static,
) -> Result<(), Box<dyn Error>> {
    let mut ui = Harness::new(app).pixels(SIZE[0], SIZE[1]);
    let image = ui.settle().image()?.ok_or("no CPU image")?;
    image.save(dir.join(format!("{name}.png")))?;
    Ok(())
}

fn staged<N: IntoNode + 'static>(
    dir: &Path,
    name: &str,
    fonts: &[(FontFamily, &'static [u8])],
    app: impl Fn() -> N + 'static,
) -> Result<(), Box<dyn Error>> {
    let mut faces = sigil::app::DefaultFonts::default();
    for (family, bytes) in fonts {
        faces.set(*family, *bytes, 0);
    }
    sigil::app::default_fonts(faces);
    let mut ui = Harness::new(app).gpu(SIZE[0], SIZE[1]).scale(SCALE);
    let image = ui.settle().image()?.ok_or("no image")?;
    image.save(dir.join(format!("{name}.png")))?;
    drop(ui);
    sigil::app::default_fonts(sigil::app::DefaultFonts::default());
    Ok(())
}

fn rolling(dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut ui = Harness::new(|| beam::lit(BEAM)).pixels(SIZE[0], SIZE[1]);
    ui.step(MID_ROLL);
    let reveal = ui.image()?.ok_or("no CPU image")?;
    reveal.save(dir.join("beam-reveal.png"))?;
    ui.settle().click("Log a session");
    let image = ui.image()?.ok_or("no CPU image")?;
    image.save(dir.join("beam-logged.png"))?;
    Ok(())
}
