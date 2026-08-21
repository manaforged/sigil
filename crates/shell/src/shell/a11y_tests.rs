#![allow(clippy::unwrap_used)]

use super::*;

fn sample_tree() -> TreeUpdate {
    TreeUpdate {
        nodes: Vec::new(),
        tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
        tree_id: accesskit::TreeId::ROOT,
        focus: accesskit::NodeId(1),
    }
}

#[test]
fn returns_none_before_any_publish() {
    let latest: LatestTree = Arc::new(Mutex::new(None));
    let mut handler = LatestTreeActivationHandler { latest };
    assert_eq!(handler.request_initial_tree(), None);
}

/// RC-focus adversarial finding 2's exact scenario: a publish lands
/// while nothing is active (simulated by writing straight into the
/// cache — real `publish_a11y_update` does the same write), then a
/// late-activating AT calls `request_initial_tree` with NO further UI
/// mutation in between. It must see the real tree, not `None`.
#[test]
fn late_activation_after_a_publish_returns_the_real_tree_not_none() {
    let latest: LatestTree = Arc::new(Mutex::new(None));
    *latest.lock().unwrap() = Some(sample_tree());
    let mut handler = LatestTreeActivationHandler { latest };

    let initial = handler.request_initial_tree();

    assert_eq!(
        initial,
        Some(sample_tree()),
        "late activation must return the cached tree, not None"
    );
}
