use std::cell::RefCell;
use std::iter::once;
use std::rc::Rc;

use crate::events::EventRouter;
use crate::events::{Event, Key, KeyPhase, Modifiers};
use crate::geometry::Size;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;
use crate::reactive::{Owner, signal};

use crate::Error;
use crate::clipboard::Clipboard;
use crate::widgets::text_field;

struct Memory(Rc<RefCell<String>>);

impl Clipboard for Memory {
    fn get(&self) -> Option<String> {
        Some(self.0.borrow().clone())
    }

    fn set(&self, text: &str) -> Result<crate::clipboard::Delivery, Error> {
        self.0.replace(text.to_string());
        Ok(crate::clipboard::Delivery::Confirmed)
    }
}

#[test]
fn shortcuts_copy_cut_paste_and_restore_unicode() {
    for modifiers in
        once(Modifiers::CTRL).chain(cfg!(target_os = "macos").then_some(Modifiers::META))
    {
        Owner::scope("field-shortcuts", || {
            let original = "café 日本語";
            let value = signal(original.to_string());
            let copied = Rc::new(RefCell::new(String::new()));
            let mut cx = Cx::new_root();
            let root = cx.add(text_field(value).clipboard(Memory(copied.clone())));
            let mut layout =
                LayoutEngine::new(Box::new(crate::layout::FakeMeasure::fixed(80.0, 20.0)));
            layout
                .compute(&mut cx, root, Size::fixed(400.0, 40.0))
                .expect("field layout");
            let mut router = EventRouter::new();
            router.set_focus(&mut cx, Some(root));
            let plain = Modifiers::empty();
            for (key, shift, expected, clipboard) in [
                ("a", plain, original, ""),
                ("c", plain, original, original),
                ("x", plain, "", original),
                ("v", plain, original, original),
                ("z", plain, "", original),
                ("Z", Modifiers::SHIFT, original, original),
            ] {
                router.route(
                    &mut cx,
                    &layout,
                    Event::key(Key::character(key), KeyPhase::Down).modifiers(modifiers | shift),
                );
                assert_eq!(value.peek(), expected, "{modifiers:?} {key}: field text");
                assert_eq!(
                    *copied.borrow(),
                    clipboard,
                    "{modifiers:?} {key}: clipboard"
                );
            }
        });
    }
}
