use crate::kernel::{Cx, container, text};
use crate::reactive::{Runtime, signal};
use crate::style::{NodeStyleExt, Paint, Token};

#[test]
fn a_colour_write_records_only_its_node_as_paint() {
    Runtime::scope("delta-colour", || {
        let lit = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().paint_when(lit, |style| style.bg(Token::Accent)));
        drop(cx.take_delta());
        lit.set(true);
        let delta = cx.take_delta();
        assert!(delta.paint.contains(&id), "{delta:?}");
        assert!(delta.layout.is_empty(), "{delta:?}");
        assert!(!delta.structure, "{delta:?}");
    });
}

#[test]
fn a_reactive_text_write_records_its_node_as_layout() {
    Runtime::scope("delta-text", || {
        let words = signal(String::from("one"));
        let mut cx = Cx::new_root();
        let id = cx.add(text(words));
        drop(cx.take_delta());
        words.set(String::from("two"));
        let delta = cx.take_delta();
        assert!(delta.layout.contains(&id), "{delta:?}");
        assert!(!delta.structure, "{delta:?}");
    });
}

#[test]
fn adding_a_child_records_structure() {
    Runtime::scope("delta-structure", || {
        let mut cx = Cx::new_root();
        let parent = cx.add(container());
        drop(cx.take_delta());
        let child = cx.add_child(parent, container());
        let delta = cx.take_delta();
        assert!(delta.structure, "{delta:?}");
        assert!(delta.layout.contains(&child), "{delta:?}");
    });
}

#[test]
fn a_frame_with_no_change_has_an_empty_delta() {
    Runtime::scope("delta-empty", || {
        let lit = signal(false);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .paint_when(lit, |style| style.bg(Token::Accent))
                .child(text("still")),
        );
        assert!(
            !cx.take_delta().is_empty(),
            "mounting {root:?} left no delta"
        );
        assert!(cx.take_delta().is_empty());
    });
}
