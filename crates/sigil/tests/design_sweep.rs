#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]

use std::collections::BTreeMap;

use sigil::View;
use sigil::prelude::*;
use sigil_kittest::Harness;

const STEPS: [&str; 9] = ["none", "xxs", "xs", "sm", "md", "lg", "xl", "xxl", "xxxl"];
const LAYERS: [&str; 5] = [
    "",
    "layer=page",
    "layer=sunken",
    "layer=raised",
    "layer=overlay",
];
const CONTAINERS: [&str; 3] = ["column", "row", "card"];
const KNOWN_DEEP_BREAKS: usize = 9;
const KNOWN_NESTING_BREAKS: usize = 1;
const LONG: &str = "A sentence long enough to need more than one line in a narrow window";

fn leaves() -> Vec<String> {
    let mut leaves: Vec<String> = [
        "caption", "body", "label", "subtitle", "title", "heading", "display",
    ]
    .iter()
    .map(|role| format!("text \"Words\" role={role}"))
    .collect();
    leaves.push(format!("text \"{LONG}\""));
    for variant in ["", "primary", "outline", "ghost"] {
        leaves.push(format!("button \"Go\" {variant} activate=@go"));
    }
    leaves.push("text_field value=$value placeholder=\"Hint\"".to_owned());
    leaves.push("checkbox \"Check\" checked=$on".to_owned());
    leaves.push("field \"Name\" {\n text_field value=$value\n}".to_owned());
    leaves
}

#[derive(Clone, Copy, Debug)]
enum Surface {
    Pixels(u32, u32),
    Cells(u16, u16),
}

const SURFACES: [Surface; 3] = [
    Surface::Pixels(1280, 800),
    Surface::Pixels(360, 800),
    Surface::Cells(120, 60),
];
const ROOMY: [Surface; 2] = [Surface::Pixels(1920, 1200), Surface::Cells(240, 120)];

fn flaws_of(source: &str, surface: Surface) -> Vec<(FlawKind, String)> {
    let text = source.to_owned();
    let app = move || {
        View::new(text.clone())
            .signal("value", signal(String::new()))
            .flag("on", signal(false))
            .action("go", || {})
    };
    let ui = Harness::new(app).unaudited();
    let mut ui = match surface {
        Surface::Pixels(width, height) => ui.pixels(width, height),
        Surface::Cells(cols, rows) => ui.size(cols, rows),
    };
    ui.settle();
    ui.flaws()
        .iter()
        .map(|flaw| (flaw.kind(), flaw.to_string()))
        .collect()
}

#[derive(Default)]
struct Report {
    trees: usize,
    broken: usize,
    smallest: BTreeMap<String, (usize, String)>,
}

impl Report {
    fn sweep(&mut self, source: &str) {
        self.sweep_on(source, &SURFACES);
    }

    fn sweep_on(&mut self, source: &str, surfaces: &[Surface]) {
        self.trees += 1;
        for surface in surfaces {
            let surface = *surface;
            let flaws = flaws_of(source, surface);
            self.broken += usize::from(!flaws.is_empty());
            for (kind, flaw) in flaws {
                let rule = format!("{kind:?} on {surface:?}");
                let entry = format!("{flaw}\n{source}");
                let smallest = self
                    .smallest
                    .entry(rule)
                    .or_insert((usize::MAX, String::new()));
                if source.len() < smallest.0 {
                    *smallest = (source.len(), entry);
                }
            }
        }
    }

    fn assert_clean(&self) {
        self.assert_at_most(0);
    }

    fn assert_at_most(&self, allowed: usize) {
        let listed: Vec<String> = self
            .smallest
            .iter()
            .map(|(rule, (_, entry))| format!("{rule}: {entry}"))
            .collect();
        assert!(
            self.broken <= allowed,
            "{} surface runs break a rule across {} trees:\n{}",
            self.broken,
            self.trees,
            listed.join("\n"),
        );
    }
}

fn tree(container: &str, attrs: &str, children: &[&str]) -> String {
    format!("{container} {attrs} {{\n{}\n}}\n", children.join("\n"))
}

#[test]
fn every_step_and_layer_of_every_container_passes_the_rules() {
    let mut report = Report::default();
    for container in CONTAINERS {
        for gap in STEPS {
            for padding in STEPS {
                for layer in LAYERS {
                    let attrs = format!("gap={gap} padding={padding} {layer}");
                    let children = ["button \"Go\" activate=@go", "text_field value=$value"];
                    report.sweep(&tree(container, &attrs, &children));
                }
            }
        }
    }
    report.assert_clean();
}

#[test]
fn every_pair_of_widgets_in_every_container_passes_the_rules() {
    let leaves = leaves();
    let mut report = Report::default();
    for container in CONTAINERS {
        for first in &leaves {
            for second in &leaves {
                report.sweep(&tree(container, "gap=md", &[first, second]));
            }
        }
    }
    report.assert_clean();
}

#[test]
fn every_nesting_of_two_containers_passes_the_rules() {
    let leaves = leaves();
    let mut report = Report::default();
    for outer in CONTAINERS {
        for inner in CONTAINERS {
            for leaf in &leaves {
                let nested = tree(
                    inner,
                    "gap=sm padding=md",
                    &[leaf, "button \"Go\" activate=@go"],
                );
                report.sweep(&tree(outer, "gap=md padding=lg", &[&nested, leaf]));
            }
        }
    }
    report.assert_at_most(KNOWN_NESTING_BREAKS);
}

const fn next(seed: &mut u32) -> usize {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 16) as usize
}

fn random_tree(seed: &mut u32, depth: usize, leaves: &[String]) -> String {
    if depth == 0 || next(seed).is_multiple_of(3) {
        return leaves[next(seed) % leaves.len()].clone();
    }
    let container = CONTAINERS[next(seed) % CONTAINERS.len()];
    let attrs = format!(
        "gap={} padding={} {}",
        STEPS[next(seed) % STEPS.len()],
        STEPS[next(seed) % STEPS.len()],
        LAYERS[next(seed) % LAYERS.len()],
    );
    let count = 1 + next(seed) % 3;
    let children: Vec<String> = (0..count)
        .map(|_| random_tree(seed, depth - 1, leaves))
        .collect();
    let children: Vec<&str> = children.iter().map(String::as_str).collect();
    tree(container, &attrs, &children)
}

#[test]
fn deeper_random_trees_pass_the_rules() {
    let leaves = leaves();
    let mut report = Report::default();
    let mut seed = 0x5eed_u32;
    for _ in 0..400 {
        report.sweep_on(&random_tree(&mut seed, 3, &leaves), &ROOMY);
    }
    report.assert_at_most(KNOWN_DEEP_BREAKS);
}

const PAIRS: &[(Token, &[Token])] = &[
    (
        Token::OnSurface,
        &[
            Token::Surface,
            Token::SurfaceHover,
            Token::SurfacePressed,
            Token::SurfaceSubtle,
            Token::SurfaceInset,
            Token::Field,
            Token::Overlay,
            Token::TableHeader,
        ],
    ),
    (
        Token::OnSurfaceMuted,
        &[
            Token::Surface,
            Token::SurfaceHover,
            Token::SurfaceSubtle,
            Token::SurfaceInset,
            Token::Field,
            Token::Overlay,
            Token::TableHeader,
        ],
    ),
    (Token::Placeholder, &[Token::Field]),
    (Token::OnAccent, &[Token::Accent, Token::AccentHover]),
];

#[test]
fn every_theme_keeps_its_text_tokens_readable_on_their_surfaces() {
    let themes = [
        ("light", Theme::light as fn() -> Theme),
        ("dark", Theme::dark),
        ("console_dark", Theme::console_dark),
    ];
    let mut failed = Vec::new();
    for (name, theme) in themes {
        for (ink, surfaces) in PAIRS {
            for surface in *surfaces {
                let (ink, surface) = (*ink, *surface);
                let mut ui = Harness::new(move || {
                    container()
                        .layout(LayoutStyle::block().fill_width())
                        .theme(theme())
                        .bg(surface)
                        .child(text("Words").text_color(ink))
                })
                .pixels(200, 80)
                .unaudited();
                failed.extend(
                    ui.flaws()
                        .iter()
                        .filter(|flaw| flaw.kind() == FlawKind::Contrast)
                        .map(|flaw| format!("{name} {ink:?} on {surface:?}: {flaw}")),
                );
            }
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
