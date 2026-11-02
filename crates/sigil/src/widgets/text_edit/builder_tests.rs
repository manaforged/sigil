use crate::events::focus::Autofocus;
use crate::kernel::Cx;
use crate::reactive::{Owner, signal};
use crate::widgets::Widget;

use super::text_edit;

#[test]
fn autofocus_reaches_the_field() {
    Owner::scope("text-edit-autofocus", || {
        let mut cx = Cx::new_root();
        let root = cx.add(text_edit(signal(String::new())).autofocus());
        assert!(
            cx.get(root)
                .expect("field")
                .get_extension::<Autofocus>()
                .is_some()
        );
    });
}
