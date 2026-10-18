use super::{TreeScope, tree, tree_scoped};
use crate::kernel::{Cx, announce, container, text};

fn spoken(update: &accesskit::TreeUpdate) -> Vec<Option<String>> {
    update
        .nodes
        .iter()
        .filter(|(_, node)| node.live().is_some())
        .map(|(_, node)| node.label().map(str::to_string))
        .collect()
}

#[test]
fn the_reading_scope_keeps_live_text_and_skips_plain_nodes() {
    crate::reactive::Runtime::scope("a11y::reading-scope", || {
        let mut cx = Cx::new_root();
        let stars = (0..40).fold(container(), |sky, star| {
            sky.child(text(format!("star {star}")))
        });
        let _ = cx.add(stars);
        announce("saved");
        cx.flush_announcements();
        let full = tree(&cx, None, &|_| None);
        let reading = tree_scoped(&cx, None, &|_| None, TreeScope::Reading);
        assert_eq!(spoken(&full), vec![Some("saved".to_string())]);
        assert_eq!(spoken(&reading), spoken(&full));
        assert_eq!(reading.focus, full.focus);
        assert!(
            reading.nodes.len() < full.nodes.len(),
            "reading projected {} of {} nodes",
            reading.nodes.len(),
            full.nodes.len()
        );
    });
}

#[test]
fn the_reading_scope_keeps_a_status_nested_in_plain_containers() {
    crate::reactive::Runtime::scope("a11y::reading-nested", || {
        let mut cx = Cx::new_root();
        let status = container().role(accesskit::Role::Status).label("saving");
        let _ = cx.add(container().child(container().child(status)));
        let full = tree(&cx, None, &|_| None);
        let reading = tree_scoped(&cx, None, &|_| None, TreeScope::Reading);
        assert_eq!(spoken(&full), vec![Some("saving".to_string())]);
        assert_eq!(spoken(&reading), spoken(&full));
    });
}
