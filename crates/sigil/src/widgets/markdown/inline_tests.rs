use crate::reactive::signal;
use crate::widgets::markdown;

#[test]
fn a_long_code_span_wraps_inside_its_column() {
    let source = "totals: `{\"books\":3499,\"food\":1025,\"rent\":145000,\"transit\":580}`";
    let grid = crate::terminal::present(move || markdown(signal(source.to_string())), 28, 10, &[])
        .expect("layout");
    assert!(grid.contains("\"transit\":580}"), "code span cut:\n{grid}");
    assert!(
        grid.lines().all(|line| line.chars().count() <= 28),
        "code span overflows:\n{grid}"
    );
}
