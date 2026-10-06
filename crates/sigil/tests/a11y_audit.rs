#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use accesskit::{Invalid, Node as AkNode, NodeId as AkId, Role as AkRole};
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;

fn find(all: &[(AkId, AkNode)], test: impl Fn(&AkNode) -> bool) -> (AkId, &AkNode) {
    all.iter()
        .find(|(_, node)| test(node))
        .map(|(id, node)| (*id, node))
        .unwrap()
}

fn parent_of(all: &[(AkId, AkNode)], child: AkId) -> Option<AkId> {
    all.iter()
        .find(|(_, node)| node.children().contains(&child))
        .map(|(id, _)| *id)
}

fn hidden_by_self_or_ancestor(all: &[(AkId, AkNode)], mut id: AkId) -> bool {
    loop {
        if all.iter().any(|(nid, node)| *nid == id && node.is_hidden()) {
            return true;
        }
        let Some(parent) = parent_of(all, id) else {
            return false;
        };
        id = parent;
    }
}

#[test]
fn a_field_error_marks_its_input_invalid_and_describes_it() {
    Runtime::scope("a11y-field", || {
        let name = signal(String::new());
        let error = signal(Some("Enter a name".to_owned()));
        let mut ui =
            Harness::new(move || column().child(field("Name", text_field(name)).error(error)));
        ui.settle();
        let all = ui.accessibility_tree().nodes;
        let (_, input) = find(&all, |node| node.role() == AkRole::TextInput);
        assert_eq!(
            input.invalid(),
            Some(Invalid::True),
            "the input is not invalid"
        );
        let described = input
            .described_by()
            .first()
            .copied()
            .expect("no described_by");
        let (_, text) = find(&all, |node| {
            all.iter()
                .any(|(id, n)| *id == described && std::ptr::eq(n, node))
        });
        let words = text
            .label()
            .or_else(|| text.value())
            .unwrap_or_default()
            .to_owned();
        assert_eq!(words, "Enter a name");
        error.set(None);
        ui.settle();
        let cleared = ui.accessibility_tree().nodes;
        let (_, valid) = find(&cleared, |node| node.role() == AkRole::TextInput);
        assert_eq!(
            valid.invalid(),
            None,
            "a cleared error leaves the input invalid"
        );
    });
}

#[test]
fn an_open_modal_is_modal_and_hides_the_page_behind_it() {
    Runtime::scope("a11y-modal", || {
        let show = signal(true);
        let mut ui = Harness::new(move || {
            column()
                .child(button("Behind"))
                .child(modal(show).title("Leave?").child(text("Unsaved changes")))
        });
        ui.settle();
        let all = ui.accessibility_tree().nodes;
        let (dialog_id, dialog) = find(&all, |node| node.role() == AkRole::Dialog);
        assert!(dialog.is_modal(), "the dialog is not marked modal");
        assert!(
            !hidden_by_self_or_ancestor(&all, dialog_id),
            "the dialog itself is hidden"
        );
        let (behind, _) = find(&all, |node| {
            node.role() == AkRole::Button && node.label() == Some("Behind")
        });
        assert!(
            hidden_by_self_or_ancestor(&all, behind),
            "the page behind the dialog is exposed"
        );
        show.set(false);
        ui.settle();
        let closed = ui.accessibility_tree().nodes;
        let (shown, _) = find(&closed, |node| {
            node.role() == AkRole::Button && node.label() == Some("Behind")
        });
        assert!(
            !hidden_by_self_or_ancestor(&closed, shown),
            "a closed dialog still hides the page"
        );
    });
}
