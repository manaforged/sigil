use super::draw;
use crate::hud::set;
use crate::profile;
use crate::terminal::paint::Grid;

#[test]
fn hud_paints_sparkline_and_last_frame() {
    let _alone = crate::hud::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    set(true);
    profile::begin_frame(profile::BackendStatus::TERMINAL);
    {
        let _span = profile::span(crate::profile::Phase::Effect);
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
    let _alone = crate::hud::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    set(true);
    profile::reset();
    let mut grid = Grid::new(80, 24);
    draw(&mut grid);
    assert!(grid.text().trim().is_empty());
    set(false);
}

#[test]
fn hud_names_the_backend_fallback() {
    use crate::profile::{BackendKind, BackendStatus, FallbackCause};
    let _alone = crate::hud::TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    set(true);
    profile::begin_frame(BackendStatus {
        kind: BackendKind::Cpu,
        fallback: FallbackCause::NoAdapter,
    });
    profile::tick_frame();
    let mut grid = Grid::new(160, 24);
    draw(&mut grid);
    set(false);
    assert!(
        grid.text().contains("[cpu (no-adapter)]"),
        "{}",
        grid.text()
    );
}
