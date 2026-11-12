use sigil::Host;
use sigil::kernel::HostMode;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::link;
use sigil_kittest::Harness;

const HREF: &str = "https://example.com/guide";
const LABEL: &str = "Read the guide";

fn page(opened: Signal<u32>) -> impl IntoNode {
    column()
        .gap(Space::Sm)
        .child(text(move || format!("Opened: {}", opened.get())))
        .child(
            link(LABEL)
                .href(HREF)
                .activate(move || opened.update(|n| *n += 1)),
        )
}

#[test]
fn a_link_runs_its_action_on_click_and_on_enter() {
    Owner::scope("link-activate", || {
        let opened = signal(0_u32);
        let mut ui = Harness::new(move || page(opened));
        ui.settle().expect_text("Opened: 0");
        let link_node = ui.get((Role::Link, LABEL));
        assert!(!link_node.is_disabled(), "link reads as enabled");
        ui.click(LABEL).settle().expect_text("Opened: 1");
        ui.focus_next().settle();
        assert_eq!(
            ui.focused().and_then(|node| node.name().map(str::to_owned)),
            Some(LABEL.to_owned()),
            "Tab reaches the link"
        );
        ui.press(NamedKey::Enter).settle().expect_text("Opened: 2");
        let tree = ui.accessibility_tree();
        let described = tree
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some(LABEL) && node.description() == Some(HREF));
        assert!(described, "screen readers get the URL as the description");
    });
}

#[test]
fn a_link_writes_its_url_into_the_terminal_cells() {
    Owner::scope("link-osc8", || {
        let opened = signal(0_u32);
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || page(opened));
        let grid = (0..4)
            .map(|_| host.grid([], 40, 4))
            .last()
            .and_then(Result::ok);
        let grid = grid.expect("terminal frame failed");
        let linked: String = (0..4)
            .flat_map(|row| (0..40).map(move |col| (col, row)))
            .filter_map(|(col, row)| grid.get(col, row))
            .filter(|cell| cell.link() == Some(HREF))
            .map(|cell| cell.text().to_owned())
            .collect();
        assert_eq!(
            linked.trim(),
            LABEL,
            "the label cells carry the OSC 8 link:\n{}",
            grid.text()
        );
    });
}
