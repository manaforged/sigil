use super::*;
use crate::kernel::{AccessibleName, Cx, IntoNode};
use crate::reactive::{Runtime, signal};

fn spoken(name: &AccessibleName) -> String {
    match name {
        AccessibleName::None => String::new(),
        AccessibleName::Static(text) => text.clone(),
        AccessibleName::Derived(text) => text.get(),
    }
}

#[test]
fn a_notice_announces_its_message() {
    Runtime::scope("notice-label", || {
        let mut cx = Cx::new_root();
        let id = notice_band("Saved to disk", Tone::Info).build(&mut cx);
        let node = cx.get(id).expect("notice node");
        assert_eq!(spoken(&node.a11y.name), "Notice: Saved to disk");
        let message = signal(String::from("first"));
        let live = notice_band(message, Tone::Error).build(&mut cx);
        message.set(String::from("second"));
        let error = cx.get(live).expect("live notice node");
        assert_eq!(spoken(&error.a11y.name), "Error: second");
    });
}
