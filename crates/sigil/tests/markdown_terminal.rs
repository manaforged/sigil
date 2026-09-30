#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used, clippy::panic)]

use sigil::Host;
use sigil::app::HostCaps;
use sigil::events::{EventRouter, ShellEvent};
use sigil::prelude::Paint;
use sigil::reactive::{Runtime, signal};
use sigil::terminal::{Cell, Grid, Underline, step};
use sigil::widgets::markdown;

const TABLE: &str = "\
| Priority | Current flavors | Existing owner | Proposed contract | Compatibility risk | Smallest bounded change |
| --- | --- | --- | --- | --- | --- |
| P1 | Definitions of tools and catalog entries spread over three separate modules with copies | tools::catalog | One catalog type that every module reads through modules::ALL at startup | Low, callers keep their names | Move runtime/configure.rs registration into the catalog and delete the copies in each module |
| P2 | Prompt flavors per provider with duplicated headers | runtime/configure.rs | Provider trait owns headers | Medium, needs a migration of the stored settings for existing users | Add the trait and port one provider first |
";

fn render(cols: u16) -> String {
    sigil::terminal::present(|| markdown(signal(TABLE.to_string())), cols, 80, &[]).expect("layout")
}

fn widths(grid: &str) -> Vec<usize> {
    let header = grid
        .lines()
        .find(|line| line.contains("Current"))
        .unwrap_or_else(|| panic!("no header in:\n{grid}"));
    let edges: Vec<usize> = header
        .chars()
        .enumerate()
        .filter_map(|(index, ch)| (ch == '│').then_some(index))
        .collect();
    edges
        .iter()
        .zip(edges.iter().skip(1))
        .map(|(left, right)| right - left - 3)
        .collect()
}

fn words() -> Vec<(usize, &'static str)> {
    TABLE
        .lines()
        .filter(|line| !line.contains("---"))
        .flat_map(|line| {
            line.trim_matches('|')
                .split('|')
                .enumerate()
                .flat_map(|(column, cell)| {
                    cell.split([' ', ',', '/'])
                        .filter(|word| !word.is_empty())
                        .map(move |word| (column, word))
                })
        })
        .collect()
}

#[test]
fn a_wide_table_keeps_words_whole_and_shares_width() {
    for cols in [230_u16, 120, 80] {
        let grid = render(cols);
        let widths = widths(&grid);
        assert_eq!(widths.len(), 6, "six columns at {cols} in:\n{grid}");
        assert!(
            grid.contains("Priority"),
            "Priority split at {cols}:\n{grid}"
        );
        for (column, word) in words() {
            if widths.get(column).is_some_and(|width| word.len() <= *width) {
                assert!(grid.contains(word), "{word} split at {cols}:\n{grid}");
            }
        }
        let text: usize = widths.iter().sum();
        assert!(
            text + 3 * 6 < usize::from(cols),
            "table wider than {cols}:\n{grid}"
        );
        let [priority, flavors, owner, _, _, change] = widths[..] else {
            panic!("six columns at {cols}: {widths:?}");
        };
        assert!(
            priority < flavors && priority < change && owner < change,
            "long columns must take the spare width at {cols}: {widths:?}\n{grid}"
        );
    }
}

const REASONING: &str = "The events are emitted to callback, but `Turn.events` doesn't include Unreported ones. For `turn_with`, `sync_usage` is called and Event type reused.";

fn paragraph(cols: u16, inline: bool) -> Vec<String> {
    let grid = sigil::terminal::present(
        move || {
            if inline {
                sigil::kernel::node(sigil::widgets::markdown_inline(REASONING).font_italic(true))
            } else {
                sigil::kernel::node(markdown(signal(REASONING.to_string())))
            }
        },
        cols,
        12,
        &[],
    )
    .expect("layout");
    let lines: Vec<String> = grid
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();
    let last = lines.iter().rposition(|line| !line.is_empty()).unwrap_or(0);
    lines.into_iter().take(last + 1).collect()
}

#[test]
fn code_spans_flow_inside_one_paragraph() {
    for cols in [200_u16, 100] {
        for inline in [true, false] {
            let rows = paragraph(cols, inline);
            let shown = rows.join("\n");
            assert!(
                rows.iter()
                    .all(|row| !row.is_empty() && !row.starts_with(' ')),
                "blank row or leading space at {cols}:\n{shown}"
            );
            let words: Vec<&str> = rows.iter().flat_map(|row| row.split_whitespace()).collect();
            let expected: Vec<&str> = REASONING
                .replace('`', "")
                .leak()
                .split_whitespace()
                .collect();
            assert_eq!(
                words, expected,
                "words lost or reordered at {cols}:\n{shown}"
            );
            if cols == 200 {
                for pair in ["but Turn.events doesn't", "For turn_with, sync_usage is"] {
                    assert!(
                        shown.contains(pair),
                        "{pair} not on one row at {cols}:\n{shown}"
                    );
                }
            }
        }
    }
}

fn grid(source: &str, cols: u16, hyperlinks: bool) -> Grid {
    let source = source.to_string();
    Runtime::scope("markdown-grid", move || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.set_caps(HostCaps { hyperlinks });
        host.mount(move || markdown(signal(source.clone())));
        step(&mut host, &mut router, Vec::<ShellEvent>::new(), cols, 24).expect("layout");
        step(&mut host, &mut router, Vec::<ShellEvent>::new(), cols, 24).expect("layout")
    })
}

fn rows(grid: &Grid) -> Vec<String> {
    grid.text()
        .lines()
        .map(|line| {
            line.replace(['\u{2060}', '\u{200b}'], "")
                .trim_end()
                .to_string()
        })
        .collect()
}

fn first(grid: &Grid, word: &str) -> Cell {
    let rows = rows(grid);
    let (row, byte) = rows
        .iter()
        .enumerate()
        .find_map(|(row, line)| line.find(word).map(|byte| (row, byte)))
        .unwrap_or_else(|| panic!("{word} missing:\n{}", rows.join("\n")));
    let col = rows
        .get(row)
        .and_then(|line| line.get(..byte))
        .map_or(0, |head| head.chars().count());
    grid.get(col, row).expect("cell")
}

#[test]
fn a_thematic_break_draws_a_rule_across_the_column() {
    let grid = grid("above\n\n---\n\nbelow\n", 40, true);
    let rows = rows(&grid);
    let shown = rows.join("\n");
    assert!(!shown.contains("---"), "literal rule in:\n{shown}");
    assert!(
        rows.iter()
            .any(|row| row.trim().chars().filter(|ch| *ch == '─').count() >= 30),
        "no rule row in:\n{shown}"
    );
}

#[test]
fn heading_levels_differ_in_the_terminal() {
    let grid = grid("# One\n\n## Two\n\n### Three\n\nbody\n", 40, true);
    let (one, two, three) = (
        first(&grid, "One"),
        first(&grid, "Two"),
        first(&grid, "Three"),
    );
    assert!(one.bold && two.bold && three.bold, "headings must be bold");
    assert_ne!(one.lines.underline, Underline::None, "h1 is underlined");
    assert_eq!(two.lines.underline, Underline::None, "h2 is not underlined");
    assert_eq!(one.fg, two.fg, "h1 and h2 share the accent");
    assert_ne!(two.fg, three.fg, "h3 drops the accent");
    assert!(!first(&grid, "body").bold, "body text is not bold");
}

const LINK: &str =
    "See <https://docs.python.org/3/library/dataclasses.html?highlight=field&mode=full> now.";

#[test]
fn a_long_url_wraps_after_a_separator() {
    let rows = rows(&grid(LINK, 30, true));
    let shown = rows.join("\n");
    let url: Vec<&String> = rows
        .iter()
        .skip_while(|row| !row.contains("https:"))
        .collect();
    assert!(
        url.first().is_some_and(|row| row.contains("https://docs.")),
        "url split after the scheme:\n{shown}"
    );
    for row in url.iter().take(2) {
        assert!(
            row.ends_with(['/', '?', '&', '=']),
            "url row ends mid-segment:\n{shown}"
        );
    }
}

#[test]
fn links_use_osc_8_or_fall_back_to_a_muted_url() {
    let source = "Read [the docs](https://example.com/a) first.";
    let linked = grid(source, 80, true);
    assert_eq!(
        first(&linked, "the").link.as_deref(),
        Some("https://example.com/a")
    );
    assert!(!rows(&linked).join("\n").contains("(https"));
    let plain = grid(source, 80, false);
    let shown = rows(&plain).join("\n");
    assert!(
        shown.contains("the docs (https://example.com/a) first."),
        "no fallback url in:\n{shown}"
    );
    assert_ne!(first(&plain, "https").fg, first(&plain, "Read").fg);
}

#[test]
fn a_wrapped_cell_keeps_every_column_border_full_height() {
    let source = "| Seat | Key env |\n| --- | --- |\n| nvidia | NVIDIA_API_KEY |\n";
    let grid = sigil::terminal::present(move || markdown(signal(source.to_string())), 22, 14, &[])
        .expect("layout");
    let body: Vec<&str> = grid
        .lines()
        .skip_while(|line| !line.contains('┌'))
        .skip(1)
        .take_while(|line| !line.contains('└'))
        .filter(|line| !line.contains('├'))
        .collect();
    assert!(body.len() > 2, "cell did not wrap:\n{grid}");
    for line in body {
        assert_eq!(
            line.matches('│').count(),
            3,
            "border gap in {line:?}:\n{grid}"
        );
    }
}
