use super::draw;

fn joined(source: &str) -> String {
    draw(source).join("\n")
}

fn column(lines: &[String], needle: &str) -> Option<(usize, usize)> {
    lines.iter().enumerate().find_map(|(row, line)| {
        line.find(needle)
            .map(|byte| (row, line.get(..byte).unwrap_or_default().chars().count()))
    })
}

#[test]
fn flowchart_top_down_draws_boxes_and_an_arrow_between_ranks() {
    let lines =
        draw("graph TD\n    A[Start] --> B{Ready?}\n    B -->|yes| C(Go)\n    B -->|no| A\n");
    let body = lines.join("\n");
    assert!(body.contains("Start"), "{body}");
    assert!(body.contains("Ready?"), "{body}");
    assert!(body.contains("Go"), "{body}");
    assert!(body.contains('▼'), "arrow head missing:\n{body}");
    assert!(body.contains("yes"), "edge label missing:\n{body}");
    let (start_row, _) = column(&lines, "Start").expect("start");
    let (ready_row, _) = column(&lines, "Ready?").expect("ready");
    let (go_row, _) = column(&lines, "Go").expect("go");
    assert!(
        start_row < ready_row && ready_row < go_row,
        "ranks stack downward:\n{body}"
    );
    assert!(
        body.contains('╱') && body.contains('╲'),
        "diamond corners:\n{body}"
    );
    assert!(body.contains('╭'), "round corners:\n{body}");
}

#[test]
fn flowchart_left_right_points_right() {
    let lines = draw("flowchart LR\n    A --> B --> C\n");
    let body = lines.join("\n");
    assert!(body.contains('▶'), "{body}");
    let (a_row, a_col) = column(&lines, "A").expect("a");
    let (c_row, c_col) = column(&lines, "C").expect("c");
    assert_eq!(a_row, c_row, "one rank row:\n{body}");
    assert!(a_col < c_col, "ranks run rightward:\n{body}");
}

#[test]
fn a_long_edge_runs_through_the_middle_rank_without_a_gap() {
    let lines = draw("graph TD\n    A --> B\n    B --> C\n    A --> C\n");
    let body = lines.join("\n");
    let (b_row, _) = column(&lines, "B").expect("b");
    let mid = lines.get(b_row).map(String::as_str).unwrap_or_default();
    assert!(
        mid.matches('│').count() >= 3,
        "a line passes beside B:\n{body}"
    );
}

#[test]
fn state_diagram_marks_start_and_end() {
    let body =
        joined("stateDiagram-v2\n    [*] --> Idle\n    Idle --> Busy: go\n    Busy --> [*]\n");
    assert!(body.contains('●') && body.contains('◉'), "{body}");
    assert!(body.contains("go"), "{body}");
}

#[test]
fn sequence_draws_lifelines_messages_notes_and_frames() {
    let source = "sequenceDiagram\n    autonumber\n    participant A as Alice\n    participant B as Bob\n    A->>B: hello there\n    B-->>A: hi\n    Note over A,B: a note\n    loop retry\n        A->>A: think\n    end\n";
    let lines = draw(source);
    let body = lines.join("\n");
    assert!(body.contains("Alice") && body.contains("Bob"), "{body}");
    assert!(body.contains("1. hello there"), "{body}");
    assert!(body.contains('▶') && body.contains('◀'), "{body}");
    assert!(body.contains("a note"), "{body}");
    assert!(body.contains("loop retry"), "{body}");
    assert!(body.contains('┆'), "lifelines:\n{body}");
    let first = lines.first().map(String::as_str).unwrap_or_default();
    let last = lines.last().map(String::as_str).unwrap_or_default();
    assert!(
        first.contains('┌') && last.contains('┘'),
        "boxes top and bottom:\n{body}"
    );
}

#[test]
fn pie_renders_shares_as_bars() {
    let body = joined("pie title Pets\n    \"Dogs\" : 60\n    \"Cats\" : 40\n");
    assert!(body.contains("Pets"), "{body}");
    assert!(body.contains("60.0%") && body.contains("40.0%"), "{body}");
    assert!(body.contains('█') && body.contains('░'), "{body}");
}

#[test]
fn unknown_kinds_are_framed_verbatim() {
    let body = joined("gantt\n    title Plan\n");
    assert!(body.contains("gantt") && body.contains('┌'), "{body}");
}

#[test]
fn pathological_source_stays_bounded() {
    let line = "a".repeat(32_768);
    let source = format!("{line}\n{}", "\n".repeat(32_768));
    let lines = draw(&source);
    assert!(lines.len() <= 600, "{} rendered lines", lines.len());
    let widest = lines.iter().map(|rendered| rendered.chars().count()).max();
    assert!(widest.is_some_and(|width| width <= 1200), "{widest:?}");
}
