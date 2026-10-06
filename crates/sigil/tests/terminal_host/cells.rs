use sigil::prelude::*;
use sigil::widgets::{ColWidth, Orientation, TableColumn, TableState, TreeNode};

use super::{present, step};

fn line_with<'a>(grid: &'a str, needle: &str) -> &'a str {
    grid.lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no {needle:?} in:\n{grid}"))
}

fn row_of(grid: &str, needle: &str) -> usize {
    grid.lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no {needle:?} in:\n{grid}"))
}

fn rows(count: usize, seed: usize) -> Vec<(usize, String)> {
    (0..count)
        .map(|at| (at, format!("job {}", at + seed)))
        .collect()
}

fn jobs_screen(active: Signal<usize>, state: &TableState<(usize, String)>) -> NodeBuilder {
    let state = state.clone();
    container()
        .layout(LayoutStyle::flex_column().fill())
        .child(
            tabs()
                .active(active)
                .tab("Jobs", move || {
                    container()
                        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
                        .child(text_field(state.quick_filter()).placeholder("filter"))
                        .child(table(state))
                })
                .tab("Other", || text("other page")),
        )
        .child(status_bar([text("ready")]))
}

fn job_table() -> TableState<(usize, String)> {
    TableState::new(
        rows(2000, 0),
        vec![
            TableColumn::new("id", "Id")
                .width(ColWidth::Fixed(64.0))
                .sort_by(|row: &(usize, String)| row.0),
            TableColumn::new("name", "Name")
                .sort_by(|row: &(usize, String)| row.1.clone())
                .cell(|row: &(usize, String)| text(row.1.clone()).nowrap()),
        ],
    )
}

#[test]
fn a_table_updated_while_its_tab_is_hidden_lays_out_when_shown() {
    sigil::reactive::Runtime::scope("hidden-table", || {
        let active = signal(0_usize);
        let state = job_table();
        let screen_state = state.clone();
        let mut host = sigil::Host::cells();
        host.mount(move || jobs_screen(active, &screen_state));
        let mut router = sigil::events::EventRouter::new();
        step(&mut host, &mut router, [], 160, 40);
        active.set(1);
        step(&mut host, &mut router, [], 160, 40);
        state.replace_rows(rows(2000, 1));
        step(&mut host, &mut router, [], 160, 40);
        active.set(0);
        let shown = sigil::terminal::step(&mut host, &mut router, [], 160, 40);
        assert!(shown.is_ok(), "showing the tab failed: {:?}", shown.err());
    });
}

#[test]
fn the_tab_strip_keeps_its_row_when_the_content_overflows() {
    sigil::reactive::Runtime::scope("tab-strip", || {
        let state = job_table();
        let grid = present(move || jobs_screen(signal(0), &state), 80, 24, &[]);
        assert!(grid.contains("Jobs") && grid.contains("Other"), "{grid}");
    });
}

#[test]
fn splitter_panes_without_a_size_share_the_width() {
    let grid = present(
        || {
            container()
                .layout(LayoutStyle::block().fill_width().height(96.0))
                .child(
                    splitter(Orientation::Horizontal)
                        .pane(text("left pane"))
                        .commit()
                        .pane(text("right pane"))
                        .commit(),
                )
        },
        60,
        8,
        &[],
    );
    assert!(
        grid.contains("left pane") && grid.contains("right pane"),
        "{grid}"
    );
}

#[test]
fn a_toggle_is_one_row_with_its_label() {
    let grid = present(|| toggle(signal(true)).label("Wi-Fi"), 40, 3, &[]);
    let line = line_with(&grid, "Wi-Fi");
    assert!(line.contains('['), "track not on the label row:\n{grid}");
    assert!(!grid.contains('█'), "toggle drew block glyphs:\n{grid}");
}

#[test]
fn a_slider_thumb_is_one_cell_not_a_box() {
    let grid = present(|| slider(signal(0.4), 0.0..=1.0), 40, 3, &[]);
    assert!(!grid.contains('╭') && !grid.contains('╰'), "{grid}");
}

#[test]
fn the_active_segment_is_marked() {
    let grid = present(
        || segmented(signal(1), vec!["Day".into(), "Week".into(), "Month".into()]),
        40,
        4,
        &[],
    );
    assert!(grid.contains("[Week]") && !grid.contains("[Day]"), "{grid}");
}

#[test]
fn a_tree_chevron_sits_on_the_folder_row() {
    let grid = present(
        || {
            tree(vec![
                TreeNode::new("src", "src").children([TreeNode::new("main", "main.rs")]),
                TreeNode::new("readme", "README.md"),
            ])
        },
        40,
        6,
        &[],
    );
    assert!(line_with(&grid, "src").contains(['▸', '▾']), "{grid}");
    assert!(
        !line_with(&grid, "README.md").contains(['▸', '▾']),
        "{grid}"
    );
}

#[test]
fn progress_draws_the_unfilled_track() {
    let grid = present(|| progress(signal(0.5_f32), 1.0), 40, 3, &[]);
    let line = line_with(&grid, "█");
    assert!(line.contains('─'), "no track after the fill:\n{grid}");
}

#[test]
fn a_status_bar_puts_its_text_on_the_row_under_its_rule() {
    let grid = present(
        || {
            column()
                .child(status_bar([text("ready")]))
                .child(text("below"))
        },
        40,
        8,
        &[],
    );
    assert!(row_of(&grid, "ready") <= 1, "{grid}");
}

#[test]
fn fixed_width_table_headers_show_their_whole_title() {
    let grid = present(
        || {
            table(TableState::new(
                vec![(1_u32, "init".to_owned())],
                vec![
                    TableColumn::new("pid", "PID")
                        .numeric()
                        .width(ColWidth::Fixed(64.0))
                        .sort_by(|row: &(u32, String)| row.0),
                    TableColumn::new("cpu", "CPU%")
                        .width(ColWidth::Fixed(64.0))
                        .sort_by(|row: &(u32, String)| row.1.clone()),
                ],
            ))
        },
        124,
        6,
        &[],
    );
    assert!(grid.contains("PID") && grid.contains("CPU%"), "{grid}");
}

#[test]
fn a_one_row_button_has_no_open_corners() {
    let grid = present(|| button("Deploy"), 20, 3, &[]);
    assert!(!grid.contains('┌') && !grid.contains('┐'), "{grid}");
}

#[test]
fn modal_actions_do_not_fuse_with_the_dialog_border() {
    let grid = present(
        || {
            modal(signal(true))
                .title("Delete file?")
                .child(text("This cannot be undone."))
                .actions(button("Cancel"), button("Delete"))
        },
        60,
        10,
        &[],
    );
    assert!(!grid.contains('┴') && !grid.contains('┬'), "{grid}");
}

#[test]
fn slider_and_progress_show_their_labels() {
    let grid = present(
        || {
            column()
                .child(slider(signal(0.4), 0.0..=1.0).label("Volume"))
                .child(progress(signal(0.5_f32), 1.0).label("Upload"))
        },
        40,
        8,
        &[],
    );
    assert!(grid.contains("Volume") && grid.contains("Upload"), "{grid}");
}
