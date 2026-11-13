use crate::events::focus::Autofocus;
use crate::kernel::Cx;
use crate::reactive::{Owner, signal};
use crate::widgets::Widget;
use crate::widgets::text_field::text_field;

#[test]
fn autofocus_reaches_the_field() {
    Owner::scope("text-edit-autofocus", || {
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(signal(String::new())).multiline().autofocus());
        assert!(
            cx.get(root)
                .expect("field")
                .extension::<Autofocus>()
                .is_some()
        );
    });
}

#[test]
fn debug_output_omits_the_text_and_its_history() {
    Owner::scope("text-edit-debug", || {
        let state = super::super::TextState::new(signal("first draft".to_owned()), signal(0));
        state.replace("second");

        let printed = format!("{state:?}");

        assert!(!printed.contains("first draft") && !printed.contains("second"));
    });
}
