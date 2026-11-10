#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]

use std::path::PathBuf;
use std::time::Duration;

use sigil::View;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

const SAVE: Duration = Duration::from_millis(400);

fn file(name: &str, text: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.sigil"));
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_view_renders_widgets_with_their_bindings() {
    let ui = || {
        let name = signal("Ada".to_owned());
        let saved = signal(0_u32);
        let status = derived(move || format!("saved {}", saved.get()));
        View::new(
            r#"
            // a settings form
            column gap=md {
              text "Settings" role=title
              field "Name" {
                text_field value=$name placeholder="Your name"
              }
              row gap=sm {
                button "Save" primary activate=@save
              }
              slot status
            }
            "#,
        )
        .signal("name", name)
        .action("save", move || saved.update(|n| *n += 1))
        .slot("status", move || text(status))
    };
    let mut ui = Harness::new(ui).size(40, 12);
    assert!(ui.text().contains("Settings"), "{}", ui.text());
    ui.expect_text("Ada");
    ui.click("Save").expect_text("saved 1");
}

#[test]
fn saving_the_file_changes_the_screen_and_keeps_signal_values() {
    let path = file("reload", "text $name\nbutton \"Old\"\n");
    let watched = path.clone();
    let mut ui = Harness::new(move || {
        View::new("text $name\nbutton \"Old\"\n")
            .watch(watched.clone())
            .signal("name", signal("Ada".to_owned()))
    })
    .size(40, 12);
    ui.expect_text("Old");
    std::fs::write(&path, "text $name\nbutton \"Brand new\"\n").unwrap();
    ui.advance(SAVE);
    let screen = ui.text();
    assert!(screen.contains("Brand new"), "{screen}");
    assert!(!screen.contains("Old"), "{screen}");
    assert!(screen.contains("Ada"), "{screen}");
}

#[test]
fn a_typed_value_survives_a_save() {
    let path = file("typed", "text_field value=$name\ntext \"One\"\n");
    let watched = path.clone();
    let name = |n: Signal<String>| derived(move || format!("typed {}", n.get()));
    let mut ui = Harness::new(move || {
        let value = signal(String::new());
        let shown = name(value);
        View::new("text_field value=$name\ntext \"One\"\n")
            .watch(watched.clone())
            .signal("name", value)
            .slot("shown", move || text(shown))
    })
    .size(40, 12);
    ui.focus_next().type_text("abc");
    std::fs::write(&path, "text_field value=$name\ntext \"Two\"\nslot shown\n").unwrap();
    ui.advance(SAVE);
    let screen = ui.text();
    assert!(screen.contains("typed abc"), "{screen}");
    assert!(screen.contains("Two"), "{screen}");
    ui.type_text("d");
    let typed = ui.text();
    assert!(typed.contains("typed abcd"), "focus was lost: {typed}");
}

#[test]
fn the_macro_reads_a_file_next_to_the_manifest() {
    let view = Owner::scope("view", || {
        sigil::view!("examples/view-form.sigil")
            .signal("name", signal(String::new()))
            .flag("updates", signal(false))
            .action("save", || {})
            .action("reset", || {})
            .slot("status", fragment)
            .check()
    });
    assert!(view.is_ok(), "{view:?}");
}

#[test]
fn a_bad_save_keeps_the_screen_and_names_the_line_then_a_good_save_clears_it() {
    let path = file("bad", "text \"Good\"\n");
    let watched = path.clone();
    let mut ui =
        Harness::new(move || View::new("text \"Good\"\n").watch(watched.clone())).size(60, 12);
    std::fs::write(&path, "text \"Good\"\nbuton \"Typo\"\n").unwrap();
    ui.advance(SAVE);
    let screen = ui.text();
    assert!(screen.contains("Good"), "{screen}");
    assert!(screen.contains("line 2 column 1"), "{screen}");
    let tree = ui.accessibility_snapshot();
    assert!(tree.contains("line 2 column 1"), "{tree}");
    std::fs::write(&path, "text \"Fixed\"\n").unwrap();
    ui.advance(SAVE);
    let fixed = ui.text();
    assert!(fixed.contains("Fixed"), "{fixed}");
    let fixed_tree = ui.accessibility_snapshot();
    assert!(!fixed_tree.contains("line 2"), "{fixed_tree}");
}

#[test]
fn a_flag_shows_and_hides_a_widget() {
    let mut ui = Harness::new(|| {
        let open = signal(false);
        View::new("button \"Toggle\" activate=@flip\ntext \"Details\" show=$open\n")
            .flag("open", open)
            .action("flip", move || open.update(|on| *on = !*on))
    })
    .size(30, 8);
    assert!(!ui.text().contains("Details"), "{}", ui.text());
    ui.click("Toggle").expect_text("Details");
}

fn problem(source: &str) -> String {
    Owner::scope("view", || {
        View::new(source)
            .signal("known", signal(String::new()))
            .check()
            .err()
            .map_or_else(String::new, |err| err.to_string())
    })
}

#[test]
fn each_mistake_names_its_place_and_the_fix() {
    let cases = [
        ("buton \"x\"", "line 1 column 1", "not a widget"),
        ("column gap=13", "line 1 column 12", "gap takes one of none"),
        (
            "column layer=glass",
            "line 1 column 14",
            "layer takes one of page",
        ),
        (
            "text \"x\" role=huge",
            "line 1 column 15",
            "role takes one of caption",
        ),
        (
            "text $missing",
            "line 1 column 6",
            "no signal named `missing`",
        ),
        (
            "button \"x\" activate=@missing",
            "line 1 column 21",
            "no action named `missing`",
        ),
        ("slot missing", "line 1 column 6", "no slot named `missing`"),
        ("column {", "line 1 column 9", "no matching `}`"),
        ("}", "line 1 column 1", "no matching `{`"),
        ("text \"open", "line 1 column 11", "no closing"),
        ("text", "line 1 column 5", "needs its text"),
        ("text_field", "line 1 column 1", "value=$name"),
        ("checkbox", "line 1 column 1", "checked=$name"),
        ("text \"x\" {\n}", "line 1 column 10", "holds no children"),
        ("field \"x\" {\n}", "line 1 column 1", "holds one control"),
        (
            "row button \"x\"",
            "line 1 column 5",
            "starts on its own line",
        ),
        (
            "column class=\"not-a-class\"",
            "line 1 column 14",
            "not-a-class",
        ),
    ];
    for (source, place, reason) in cases {
        let found = problem(source);
        assert!(found.contains(place), "{source}: {found}");
        assert!(found.contains(reason), "{source}: {found}");
    }
}

#[test]
fn nesting_past_the_limit_is_a_problem_and_not_a_crash() {
    let deep = "column {\n".repeat(100_000);
    assert!(problem(&deep).contains("nest deeper"));
}

#[test]
fn any_bytes_give_a_problem_or_a_view_and_never_a_panic() {
    const PIECES: [&str; 14] = [
        "column", "text", "\"x", "\"y\"", "$", "$known", "@", "=", "{", "}", "\n", "gap", "é", "\\",
    ];
    let mut seed = 0x9e37_79b9_u32;
    for _ in 0..4_000 {
        let mut source = String::new();
        for _ in 0..(seed % 12) {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            source.push_str(PIECES[(seed >> 16) as usize % PIECES.len()]);
            source.push(' ');
        }
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let _ = problem(&source);
    }
}

#[test]
#[should_panic(expected = "design flaws")]
fn a_view_is_held_to_the_design_rules() {
    let mut ui =
        Harness::new(|| View::new("row {\n  button \"Hi\" class=\"h-4\"\n}\n")).pixels(360, 160);
    ui.settle();
}
