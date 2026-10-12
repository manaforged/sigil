use super::*;

use crate::kernel::NodeType;
use crate::reactive::Runtime;

fn dialog_of(cx: &Cx, root: crate::kernel::NodeId) -> crate::kernel::NodeId {
    cx.get(root)
        .expect("root")
        .children()
        .iter()
        .copied()
        .find(|id| matches!(cx.get(*id).expect("child").a11y.role, Role::Dialog))
        .expect("dialog")
}

#[test]
fn sheet_docks_a_dialog_on_the_right() {
    Runtime::scope("sheet-right", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(sheet(show).title("Filters"));
        let frame = dialog_of(&cx, root);
        let style = cx
            .get(frame)
            .expect("frame")
            .get_extension::<LayoutStyle>()
            .expect("layout")
            .clone();
        assert_eq!(style.data().size.width, crate::layout::Dimension::Px(360.0));
        assert_eq!(style.data().size.height, crate::layout::Dimension::Fill);
    });
}

#[test]
fn signal_title_drives_dialog_text_and_accessible_name() {
    Runtime::scope("modal-signal-title", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let title = signal(String::from("Create entry"));
        let root = cx.add(modal(show).title(title));

        let frame = dialog_of(&cx, root);
        let name = match &cx.get(frame).expect("frame").a11y.name {
            AccessibleName::Derived(name) => *name,
            other => unreachable!("signal title must map to a derived name, got {other:?}"),
        };
        assert_eq!(name.get(), "Create entry");

        let title_node = cx
            .get(frame)
            .expect("frame")
            .children()
            .iter()
            .flat_map(|id| cx.get(*id).expect("child").children().iter().copied())
            .find(|id| matches!(cx.get(*id).expect("child").kind, NodeType::Text { .. }))
            .expect("title text node");
        let title_text = match &cx.get(title_node).expect("title").kind {
            NodeType::Text { content, .. } => content.clone(),
            other => unreachable!("expected NodeType::Text, got {other:?}"),
        };
        assert_eq!(title_text.resolve(), "Create entry");

        title.set(String::from("Edit entry"));
        assert_eq!(name.get(), "Edit entry");
        assert_eq!(
            title_text.resolve(),
            "Edit entry",
            "title bar text follows the signal"
        );
    });
}
