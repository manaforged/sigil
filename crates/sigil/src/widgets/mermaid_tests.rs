use super::draw;
use crate::layout::LayoutBuilderExt;

fn joined(source: &str) -> String {
    draw(source).join("\n")
}

fn assert_has(body: &str, needles: &[&str]) {
    for needle in needles {
        assert!(body.contains(needle), "{needle} missing:\n{body}");
    }
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
    assert_has(&body, &["Start", "Ready?", "Go", "▼", "yes", "╱", "╲", "╭"]);
    let (start_row, _) = column(&lines, "Start").expect("start");
    let (ready_row, _) = column(&lines, "Ready?").expect("ready");
    let (go_row, _) = column(&lines, "Go").expect("go");
    assert!(
        start_row < ready_row && ready_row < go_row,
        "ranks stack downward:\n{body}"
    );
}

#[test]
fn flowchart_left_right_points_right() {
    let lines = draw("flowchart LR\n    A --> B --> C\n");
    let body = lines.join("\n");
    assert_has(&body, &["▶"]);
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
    assert_has(&body, &["●", "◉", "go"]);
}

#[test]
fn sequence_draws_lifelines_messages_notes_and_frames() {
    let source = "sequenceDiagram\n    autonumber\n    participant A as Alice\n    participant B as Bob\n    A->>B: hello there\n    B-->>A: hi\n    Note over A,B: a note\n    loop retry\n        A->>A: think\n    end\n";
    let lines = draw(source);
    let body = lines.join("\n");
    assert_has(
        &body,
        &[
            "Alice",
            "Bob",
            "1. hello there",
            "▶",
            "◀",
            "a note",
            "loop retry",
            "┆",
        ],
    );
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
    assert_has(&body, &["Pets", "60.0%", "40.0%", "█", "░"]);
}

#[test]
fn unknown_kinds_are_framed_verbatim() {
    let body = joined("gantt\n    title Plan\n");
    assert_has(&body, &["gantt", "┌"]);
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

const TURN: &str = r"sequenceDiagram
    participant S as summonr
    participant C as Claude Code process
    participant M as Model

    S->>S: Lock session; take reusable parked run
    alt No reusable process
        S->>S: Choose new session or resume known thread
        S->>C: Spawn with stream-json stdin/stdout,<br/>model, system prompt, effort, SDK MCP server<br/>(built-in tools disabled)
        S->>C: control_request: initialize
        opt MCP setup requests
            C->>S: control_request: mcp_message<br/>(initialize / tools/list)
            S->>C: control_response: mcp_response<br/>(capabilities / summonr tool definitions)
        end
        C-->>S: control_response: initialize success
        S->>S: Cache available model listing
        S->>C: user JSON message(s)<br/>(replay earlier history only for new thread)
    else Reusable process
        S->>C: Pending tool results and/or new user message(s)
    end

    C->>M: Request completion with conversation and tools
    loop Streaming response
        M-->>C: Text / thinking deltas
        C-->>S: stream_event: content_block_delta
        S->>S: Accumulate partials; emit deltas
    end
    M-->>C: Assistant response
    C-->>S: assistant message with usage

    alt Model requests a summonr tool
        C->>S: control_request: can_use_tool
        S->>C: control_response: allow summonr tool
        C->>S: control_request: mcp_message tools/call
        S->>S: Queue pending call; return Paused<br/>AssistantMessage with tool_calls
        Note over S,C: Process remains alive, waiting for tool results.<br/>A subsequent completion resumes this run.
    else Model finishes
        C-->>S: result with status and usage
        S->>S: Return Done AssistantMessage
    end

    S->>S: Remember observed thread; park run; unlock session";

#[test]
fn a_real_sequence_diagram_fits_a_terminal_column() {
    let lines = draw(TURN);
    let widest = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= 120,
        "{widest} columns wide:\n{}",
        lines.join("\n")
    );
    let body = lines.join("\n");
    assert!(!body.contains("<br"), "{body}");
    assert_has(
        &body,
        &[
            "Claude Code process",
            "(built-in tools disabled)",
            "Return Done",
        ],
    );
}

#[test]
fn a_diagram_wider_than_its_column_keeps_one_row_per_line() {
    let source = "graph LR\n    A[first stage of the pipeline] --> B[second stage of the pipeline] --> C[third stage]";
    let drawn = draw(source).len();
    let grid = crate::terminal::present(
        move || {
            crate::kernel::container()
                .layout(crate::layout::LayoutStyle::flex_column().fill_width())
                .child(super::figure(source))
        },
        40,
        30,
        &[],
    )
    .expect("layout");
    let rows = grid.lines().filter(|line| !line.trim().is_empty()).count();
    assert_eq!(rows, drawn, "{grid}");
}
