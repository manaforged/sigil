use super::*;

#[test]
fn the_tree_paints_a_header_and_a_frame() {
    let reader = Arc::new(Mutex::new(scan::Scanner::new()));
    let first = reader.lock().map(|mut r| r.read()).unwrap_or_default();
    let grid = sigil_tui::present(
        move |cx: &mut Cx| {
            let state = AppState::adopt(
                Arc::clone(&reader),
                first.clone(),
                scan::dev_root(),
                scan::LIMIT,
            );
            cx.add(view::app_tree(&state))
        },
        120,
        32,
        &[],
    )
    .expect("layout");
    println!("{grid}");
    assert!(grid.contains("PID"), "expected a PID header in:\n{grid}");
    assert!(
        grid.contains("COMMAND"),
        "expected a COMMAND header in:\n{grid}"
    );
    assert!(grid.contains("strays"), "expected the summary in:\n{grid}");
    assert!(
        grid.contains("Refresh"),
        "expected the action bar in:\n{grid}"
    );
}
