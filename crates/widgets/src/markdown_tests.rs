use sigil_kernel::Cx;
use sigil_reactive::{Runtime, signal};

use super::{markdown, markdown_edit};

#[test]
fn markdown_builds() {
    Runtime::scope("markdown-builds", || {
        let source = signal(String::from("# Hi\n\nthere"));
        let mut cx = Cx::new_root();
        let id = cx.add(markdown(source));
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn markdown_edit_builds() {
    Runtime::scope("markdown-edit-builds", || {
        let source = signal(String::from("# Hi"));
        let mut cx = Cx::new_root();
        let id = cx.add(markdown_edit(source));
        assert!(cx.get(id).is_some());
    });
}
