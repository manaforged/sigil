mod egui_rig;
#[cfg(feature = "iced")]
mod iced_rig;
mod measure;
mod ratatui_rig;
mod scene;
mod sigil_rig;
#[cfg(feature = "slint")]
mod slint_rig;

use egui_rig::{EguiRig, Mode};
use measure::{Counting, run};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui_rig::{Extent, RatatuiRig};
use sigil_rig::{SigilRig, Surface, Tree};

#[global_allocator]
static GLOBAL: Counting = Counting;

type Buffered = RatatuiRig<TestBackend>;
type Written = RatatuiRig<CrosstermBackend<Vec<u8>>>;
type Side = (&'static str, fn(&str, Option<&str>));

const SIDES: &[Side] = &[
    ("sigil-cell", |side, only| {
        run(side, only, &|| SigilRig::new(Surface::Grid, Tree::Scroll));
    }),
    ("sigil-cell-screen", |side, only| {
        run(side, only, &|| SigilRig::new(Surface::Screen, Tree::Scroll));
    }),
    ("sigil-cell-virtual", |side, only| {
        run(side, only, &|| SigilRig::new(Surface::Screen, Tree::Virtual));
    }),
    ("ratatui", |side, only| {
        run(side, only, &|| Buffered::new(Extent::Visible));
    }),
    ("ratatui-whole", |side, only| {
        run(side, only, &|| Buffered::new(Extent::Whole));
    }),
    ("ratatui-ansi", |side, only| {
        run(side, only, &|| Written::new(Extent::Visible));
    }),
    ("ratatui-whole-ansi", |side, only| {
        run(side, only, &|| Written::new(Extent::Whole));
    }),
    ("sigil-pixel", |side, only| {
        run(side, only, &|| SigilRig::new(Surface::Scene, Tree::Scroll));
    }),
    ("sigil-pixel-virtual", |side, only| {
        run(side, only, &|| SigilRig::new(Surface::Scene, Tree::Virtual));
    }),
    ("egui", |side, only| {
        run(side, only, &|| EguiRig::new(Mode::All));
    }),
    ("egui-shapes", |side, only| {
        run(side, only, &|| EguiRig::new(Mode::Shapes));
    }),
    ("egui-rows", |side, only| {
        run(side, only, &|| EguiRig::new(Mode::Rows));
    }),
    #[cfg(feature = "iced")]
    ("iced", |side, only| {
        run(side, only, &iced_rig::IcedRig::new);
    }),
    #[cfg(feature = "slint")]
    ("slint", |side, only| {
        run(side, only, &|| slint_rig::SlintRig::new(false));
    }),
    #[cfg(feature = "slint")]
    ("slint-list", |side, only| {
        run(side, only, &|| slint_rig::SlintRig::new(true));
    }),
];

fn main() {
    let mut args = std::env::args().skip(1);
    let wanted = args.next().unwrap_or_default();
    let only = args.next();
    match SIDES.iter().find(|(name, _)| *name == wanted) {
        Some((name, cells)) => cells(name, only.as_deref()),
        None => {
            let names: Vec<&str> = SIDES.iter().map(|(name, _)| *name).collect();
            eprintln!("usage: rivals <{}> [cell]", names.join("|"));
            std::process::exit(2);
        }
    }
}
