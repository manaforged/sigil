#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use sigil::prelude::*;
use sigil::widgets::Highlight;
use sigil_kittest::Harness;

const STACK: usize = 8 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(30);

fn mounts_within_deadline(name: &str, build: fn() -> NodeBuilder) -> bool {
    let (done, finished) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name(name.to_string())
        .stack_size(STACK)
        .spawn(move || {
            let mut ui = Harness::new(move || container().child(build())).pixels(800, 600);
            ui.settle();
            done.send(()).expect("receiver");
        })
        .expect("spawn");
    let returned = finished.recv_timeout(DEADLINE).is_ok();
    if returned {
        worker.join().expect("mount thread panicked");
    }
    returned
}

#[test]
fn deeply_nested_dot_braces_fall_back_without_overflowing_the_stack() {
    let returned = mounts_within_deadline("dot-nesting", || {
        let source = format!("digraph G {{\n{}", ("{".repeat(500) + "\n").repeat(200));
        graphviz(signal(source))
    });
    assert!(returned, "the DOT source never returned");
}

#[test]
fn a_gantt_with_an_absurd_duration_mounts_in_bounded_time() {
    let returned = mounts_within_deadline("gantt-duration", || {
        mermaid(signal(
            "gantt\n  dateFormat YYYY-MM-DD\n  section S\n  A :a1, 2024-01-01, 1e15d\n".to_string(),
        ))
    });
    assert!(
        returned,
        "the gantt source did not mount within {DEADLINE:?}"
    );
}

#[test]
fn a_source_only_the_vector_renderer_draws_replaces_the_old_picture() {
    let slot: Rc<Cell<Option<Signal<String>>>> = Rc::new(Cell::new(None));
    let export = Rc::clone(&slot);
    let mut ui = Harness::new(move || {
        let source = signal("flowchart TD\n  A-->B\n".to_string());
        export.set(Some(source));
        container().child(mermaid(source))
    })
    .pixels(800, 600);
    ui.settle();
    slot.get()
        .expect("source signal")
        .set("flowchart TD\n  A-->B\n  B-->C(\n".to_string());
    ui.settle();
    let shown = ui.text();
    assert!(!shown.contains("Updating"), "stale picture shown:\n{shown}");
}

fn marks(language: &str, source: &str) -> Vec<Vec<Highlight>> {
    highlight(language, source)
}

fn has_token(line: &[Highlight], token: Token) -> bool {
    line.iter().any(|mark| mark.token() == token)
}

#[test]
fn a_lua_block_comment_wins_over_the_line_comment() {
    let lines = marks("lua", "--[[ start\nstill comment\n]]\nlocal x = 1");
    let inner = &lines[1];
    assert!(
        inner.iter().any(|mark| mark.start() == 0
            && mark.end() == 13
            && mark.token() == Token::OnSurfaceMuted),
        "line 2 is not a comment: {inner:?}"
    );
    assert!(
        !has_token(&lines[3], Token::OnSurfaceMuted),
        "{:?}",
        lines[3]
    );
}

#[test]
fn an_elixir_map_is_not_a_comment() {
    let lines = marks("elixir", "m = %{a: 1}");
    assert!(
        !has_token(&lines[0], Token::OnSurfaceMuted),
        "{:?}",
        lines[0]
    );
    let erlang = marks("erlang", "% note");
    assert!(
        has_token(&erlang[0], Token::OnSurfaceMuted),
        "{:?}",
        erlang[0]
    );
}

#[test]
fn a_powershell_static_member_is_not_a_comment() {
    let lines = marks("powershell", "$r = [Math]::Round(2.5)");
    assert!(
        !has_token(&lines[0], Token::OnSurfaceMuted),
        "{:?}",
        lines[0]
    );
    let block = marks("powershell", "<# note #> $x");
    assert!(
        has_token(&block[0], Token::OnSurfaceMuted),
        "{:?}",
        block[0]
    );
}

#[test]
fn a_julia_transpose_does_not_open_a_string() {
    let lines = marks("julia", "y = A' * B + 1");
    assert!(
        !has_token(&lines[0], Tone::Success.step(11)),
        "{:?}",
        lines[0]
    );
    assert!(
        !has_token(&lines[0], Token::OnSurfaceMuted),
        "{:?}",
        lines[0]
    );
}

#[test]
fn an_fsharp_type_variable_does_not_open_a_string() {
    let lines = marks("fsharp", "let id (x: 'a) = x");
    assert!(
        !has_token(&lines[0], Tone::Success.step(11)),
        "{:?}",
        lines[0]
    );
    assert!(
        lines[0]
            .iter()
            .any(|mark| mark.start() == 0 && mark.end() == 3)
    );
}

const NEW_KINDS: [(&str, &str); 4] = [
    (
        "venn-beta\n  set Desirable\n  set Feasible\n  union Desirable,Feasible[\"Buildable\"]\n",
        "union Desirable",
    ),
    (
        "wardley-beta\ntitle Tea Shop\nanchor Business [0.95, 0.63]\ncomponent Tea [0.63, 0.81]\nBusiness -> Tea\n",
        "anchor Business",
    ),
    ("railroad-ebnf-beta\ndigit = \"0\" | \"1\" ;\n", "digit ="),
    (
        "agentflow-beta TB\n  flow reviewer[\"Review Agent\"]\n    changes[\"Gather changes\"]@{ shape: input }\n    lint[\"run_linter\"]@{ shape: tool }\n    changes --> lint\n  end\n",
        "flow reviewer",
    ),
];

#[test]
fn newer_mermaid_kinds_draw_a_diagram_instead_of_their_source() {
    for (source, raw) in NEW_KINDS {
        let mut ui = Harness::new(move || container().child(mermaid(source))).pixels(800, 600);
        ui.settle();
        let shown = ui.text();
        assert!(
            shown.contains("Name=\"Show code\""),
            "{raw:?} fell back to source text:\n{shown}"
        );
    }
}
