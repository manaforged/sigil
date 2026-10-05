#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::widgets::TreeNode;
use sigil_kittest::{Harness, Query};

#[test]
fn tree_expands_collapses_and_activates_leaves() {
    Runtime::scope("widget-gaps-tree", || {
        let opened = signal(None::<&'static str>);
        let nodes = vec![
            TreeNode::new("src", "src").children([TreeNode::new("main", "main.rs")]),
            TreeNode::new("readme", "README.md"),
        ];
        let mut ui =
            Harness::new(move || tree(nodes.clone()).activate(move |id| opened.set(Some(*id))));
        assert!(
            ui.query(&Query::text("main.rs")).is_none(),
            "folders start closed"
        );
        ui.click("src").settle();
        assert!(
            ui.query(&Query::text("main.rs")).is_some(),
            "a click opens a folder"
        );
        ui.press(Key::ArrowLeft).settle();
        assert!(
            ui.query(&Query::text("main.rs")).is_none(),
            "Left closes the folder"
        );
        ui.press(Key::ArrowRight).settle();
        ui.click("main.rs").settle();
        assert_eq!(opened.get(), Some("main"), "a click on a leaf activates it");
    });
}

#[test]
fn card_activates_on_click_and_enter() {
    Runtime::scope("widget-gaps-card", || {
        let hits = signal(0_u32);
        let mut ui =
            Harness::new(move || card(text("Deploy")).activate(move || hits.update(|n| *n += 1)));
        ui.click("Deploy").settle();
        assert_eq!(hits.get(), 1);
        ui.focus_next().press(Key::Enter).settle();
        assert_eq!(hits.get(), 2);
    });
}

#[test]
fn stat_shows_label_value_delta_and_caption() {
    let mut ui = Harness::new(|| {
        stat("Success rate", "96.4%")
            .delta("+2.1%", Tone::Success)
            .caption("last 7 days")
    });
    ui.expect_text("Success rate")
        .expect_text("96.4%")
        .expect_text("+2.1%")
        .expect_text("last 7 days");
}

#[test]
fn kv_grid_values_follow_their_signals() {
    Runtime::scope("widget-gaps-kv", || {
        let state = signal("queued".to_owned());
        let mut ui = Harness::new(move || kv_grid([("State", move || state.get())]));
        ui.expect_text("queued");
        state.set("live".to_owned());
        ui.settle().expect_text("live");
    });
}

#[test]
fn clearable_field_empties_the_value_and_fires_change() {
    Runtime::scope("widget-gaps-clear", || {
        let value = signal(String::new());
        let changes = signal(0_u32);
        let mut ui = Harness::new(move || {
            text_field(value)
                .placeholder("Search")
                .leading(text("@"))
                .clearable()
                .change(move |_| changes.update(|n| *n += 1))
        });
        assert!(ui.query(&Query::role(Role::Button).name("Clear")).is_none());
        let field = ui.get_by_role(Role::TextInput, "Search");
        ui.click(&field).type_text("ana").settle();
        assert_eq!(value.get(), "ana");
        let before = changes.get();
        let clear = ui.get_by_role(Role::Button, "Clear");
        let filled = ui.get_by_role(Role::TextInput, "Search");
        let (outer, inner) = (filled.bounds().unwrap(), clear.bounds().unwrap());
        assert!(
            inner.origin.x >= outer.origin.x
                && inner.origin.x + inner.width <= outer.origin.x + outer.width,
            "the clear button sits inside the field border",
        );
        ui.click(&clear).settle();
        assert_eq!(value.get(), "", "Clear empties the value");
        assert!(changes.get() > before, "Clear fires change");
        assert!(
            ui.query(&Query::role(Role::Button).name("Clear")).is_none(),
            "Clear hides once empty"
        );
    });
}

#[test]
fn list_row_draws_its_badge() {
    let mut ui = Harness::new(|| {
        list_row_with(
            false,
            "Incidents",
            ListRowSlots {
                badge: Some(("12".into(), Tone::Error)),
                ..ListRowSlots::default()
            },
        )
    });
    ui.expect_text("Incidents").expect_text("12");
}

#[test]
fn menu_bar_is_as_wide_as_its_menus() {
    let mut ui = Harness::new(|| {
        row()
            .child(menu_bar().menu("File", [MenuItem::new("Open").activate(|| {})]))
            .child(text("after"))
    })
    .pixels(800, 200);
    let after = ui.get_by_text("after").bounds().unwrap();
    assert!(
        after.origin.x < 400.0,
        "text after the menu bar sits at x {}",
        after.origin.x
    );
}

#[test]
fn tab_content_fills_the_space_below_the_strip() {
    let mut ui = Harness::new(|| {
        column()
            .layout(LayoutStyle::flex_column().fixed_size(600.0, 400.0))
            .child(tabs().tab("One", || {
                container()
                    .role(Role::Group)
                    .label("body")
                    .layout(LayoutStyle::block().fill())
            }))
    })
    .pixels(600, 400);
    let body = ui.get_by_role(Role::Group, "body").bounds().unwrap();
    assert!(
        body.height > 300.0,
        "tab content is {} px tall in a 400 px frame",
        body.height
    );
}

#[test]
fn chart_fills_its_parent_width() {
    let mut ui = Harness::new(|| {
        column()
            .layout(LayoutStyle::flex_column().fixed_size(600.0, 300.0))
            .child(chart(vec![1.0, 3.0, 2.0]).height(120.0))
    })
    .pixels(600, 300);
    let figure = ui.get(&Query::role(Role::Figure)).bounds().unwrap();
    assert!(
        figure.width > 500.0,
        "chart is {} px wide in a 600 px parent",
        figure.width
    );
}

#[test]
fn highlight_marks_markdown_headings_and_typescript_keywords() {
    let heading = highlight("markdown", "# Title");
    assert!(
        heading.first().is_some_and(|line| !line.is_empty()),
        "markdown heading has a mark"
    );
    let ts = highlight("ts", "interface Point { x: number }");
    assert!(
        ts.first().is_some_and(|line| !line.is_empty()),
        "typescript keyword has a mark"
    );
}

#[test]
fn every_bundled_icon_bakes_to_visible_pixels() {
    let oven = sigil::icons::IconBaker::new();
    for icon in Icon::ALL {
        let baked = oven
            .bake(icon.svg(), 24, 24)
            .unwrap_or_else(|err| panic!("{} does not bake: {err:?}", icon.name()));
        assert!(
            baked.alpha.iter().any(|a| *a > 0),
            "{} bakes empty",
            icon.name()
        );
    }
}

#[test]
fn scroll_converts_into_a_node_builder() {
    let mut ui = Harness::new(|| {
        let node: NodeBuilder = scroll(container().child(text("body"))).into();
        column().child(node)
    });
    ui.expect_text("body");
}

#[test]
fn divider_vertical_takes_a_fixed_height() {
    Runtime::scope("widget-gaps-divider", || {
        let mut cx = sigil::kernel::Cx::new_root();
        let root = cx.add(
            row()
                .layout(LayoutStyle::flex_row().fixed_size(200.0, 100.0))
                .child(divider_vertical().height(16.0)),
        );
        let mut layout =
            sigil::layout::LayoutEngine::with_text_measure(Box::new(sigil_kittest::FixedMeasure {
                width: 10.0,
                height: 16.0,
            }));
        layout
            .compute(&mut cx, root, sigil::geometry::Size::fixed(200.0, 100.0))
            .unwrap();
        let divider = cx
            .get(root)
            .and_then(|node| node.children().first().copied())
            .unwrap();
        let rect = layout.result(divider).unwrap().rect();
        assert!(
            (rect.height - 16.0).abs() < 0.5,
            "divider is {} px tall",
            rect.height
        );
        assert!(
            (rect.width - 1.0).abs() < 0.5,
            "divider is {} px wide",
            rect.width
        );
    });
}

#[test]
fn square_icon_button_draws_its_icon_not_an_empty_label() {
    Runtime::scope("widget-gaps-icon-button", || {
        let mut cx = sigil::kernel::Cx::new_root();
        let root = cx.add(icon_button("", || {}).icon(Icon::Search));
        let mut texts = 0;
        cx.visit_preorder(root, |_, node| {
            if matches!(node.kind, sigil::kernel::NodeType::Text { .. }) {
                texts += 1;
            }
        });
        assert_eq!(texts, 0, "a square icon button holds no text label");
    });
}

#[test]
fn chip_dot_follows_a_reactive_tone() {
    Runtime::scope("widget-gaps-chip-dot", || {
        let tone = signal(Tone::Success);
        let mut cx = sigil::kernel::Cx::new_root();
        let root = cx.add(chip("Live", Tone::Success).tone(tone).dot());
        let mut layout =
            sigil::layout::LayoutEngine::with_text_measure(Box::new(sigil_kittest::FixedMeasure {
                width: 10.0,
                height: 16.0,
            }));
        let size = sigil::geometry::Size::fixed(200.0, 40.0);
        layout.compute(&mut cx, root, size).unwrap();
        tone.set(Tone::Error);
        layout.compute(&mut cx, root, size).unwrap();
        let mut tones = Vec::new();
        cx.visit_preorder(root, |_, node| {
            if let Some(context) = node.get_extension::<sigil::style::ToneContext>() {
                tones.push(context.tone);
            }
        });
        assert!(
            tones.contains(&Tone::Error),
            "the dot takes the chip's new tone"
        );
        assert!(
            !tones.contains(&Tone::Success),
            "no dot keeps the old tone: {tones:?}"
        );
    });
}

#[test]
fn tree_indent_sets_the_nesting_step() {
    let nested_x = |indent: Option<f32>| {
        let mut ui = Harness::new(move || {
            let nodes =
                vec![TreeNode::new("src", "src").children([TreeNode::new("main", "main.rs")])];
            let open = signal(vec!["src"]);
            let built = tree(nodes).expanded(open);
            match indent {
                Some(cols) => built.indent(cols),
                None => built,
            }
        })
        .pixels(400, 200);
        ui.find_all(&Query::text("main.rs"))
            .into_iter()
            .filter(|node| node.role() != Role::TreeItem)
            .find_map(|node| node.bounds())
            .unwrap()
            .origin
            .x
    };
    assert!(
        nested_x(Some(6.0)) > nested_x(None) + 20.0,
        "a wider indent moves nested rows right"
    );
}
