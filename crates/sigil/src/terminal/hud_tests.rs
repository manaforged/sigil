use super::draw;
use crate::hud::set;
use crate::profile;
use crate::terminal::paint::Grid;

#[test]
fn hud_paints_sparkline_and_last_frame() {
    set(true);
    profile::begin_frame();
    {
        let _span = profile::span("effect");
    }
    profile::tick_frame();
    let mut grid = Grid::new(80, 24);
    draw(&mut grid);
    let text = grid.text();
    let first = text.lines().next().unwrap_or_default();
    let second = text.lines().nth(1).unwrap_or_default();
    assert!(first.trim_end().ends_with('▁'), "sparkline row: {first:?}");
    assert!(second.contains("idle"), "stats row: {second:?}");
    set(false);
}

#[test]
fn hud_stays_blank_without_frames() {
    set(true);
    profile::reset();
    let mut grid = Grid::new(80, 24);
    draw(&mut grid);
    assert!(grid.text().trim().is_empty());
    set(false);
}
