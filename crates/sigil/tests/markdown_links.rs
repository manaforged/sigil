#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

use accesskit::Role as AkRole;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;
use sigil_kittest::input::click;

const HREF: &str = "https://example.com/guide";

#[test]
fn a_markdown_link_has_an_area_and_opens_on_click() {
    Runtime::scope("markdown-links", || {
        let opened = signal(Vec::<String>::new());
        let source = signal(format!("Read [the guide]({HREF}) before you start."));
        let mut ui = Harness::new(move || {
            column()
                .link(move |href| opened.update(|seen| seen.push(href.to_owned())))
                .child(markdown(source))
        })
        .pixels(600, 200);
        ui.settle();
        let nodes = ui.accessibility_tree().nodes;
        let link = nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == AkRole::Link)
            .expect("no Link node");
        assert_eq!(link.label(), Some("the guide"));
        assert_eq!(link.url(), Some(HREF));
        let area = link.bounds().expect("link bounds");
        assert!(
            area.width() > 10.0 && area.height() > 5.0,
            "link area {area:?}"
        );
        let (x, y) = (area.x0.midpoint(area.x1), area.y0.midpoint(area.y1));
        ui.send(click(x as f32, y as f32));
        ui.settle();
        assert_eq!(opened.peek(), vec![HREF.to_owned()]);
    });
}
