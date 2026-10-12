use std::cell::RefCell;
use std::iter::once;
use std::rc::Rc;

use crate::events::EventRouter;
use crate::events::{
    KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState, PhysicalKey, ShellEvent,
};
use crate::geometry::{LogicalSize, Size};
use crate::kernel::{Cx, TextContent};
use crate::layout::{LayoutEngine, TextMeasure};
use crate::reactive::{Runtime, signal};

use crate::clipboard::{ClipError, Clipboard};
use crate::widgets::text_field;

struct Memory(Rc<RefCell<String>>);

impl Clipboard for Memory {
    fn get(&self) -> Option<String> {
        Some(self.0.borrow().clone())
    }

    fn set(&self, text: &str) -> Result<crate::clipboard::Delivery, ClipError> {
        self.0.replace(text.to_string());
        Ok(crate::clipboard::Delivery::Confirmed)
    }
}

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _source: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(80.0, 20.0)
    }
}

#[test]
fn shortcuts_copy_cut_paste_and_restore_unicode() {
    for modifiers in once(ModifiersState::CONTROL)
        .chain(cfg!(target_os = "macos").then_some(ModifiersState::META))
    {
        Runtime::scope("field-shortcuts", || {
            let original = "café 日本語";
            let value = signal(original.to_string());
            let copied = Rc::new(RefCell::new(String::new()));
            let mut cx = Cx::new_root();
            let root = cx.add(text_field(value).clipboard(Memory(copied.clone())));
            let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
            layout
                .compute(&mut cx, root, Size::fixed(400.0, 40.0))
                .expect("field layout");
            let mut router = EventRouter::new();
            router.set_focus(&mut cx, Some(root));
            let plain = ModifiersState::empty();
            for (key, shift, expected, clipboard) in [
                ("a", plain, original, ""),
                ("c", plain, original, original),
                ("x", plain, "", original),
                ("v", plain, original, original),
                ("z", plain, "", original),
                ("Z", ModifiersState::SHIFT, original, original),
            ] {
                router.route(
                    &mut cx,
                    &layout,
                    ShellEvent::Key(KeyEvent::new(KeyEventInit {
                        key: LogicalKey::character(key),
                        physical: PhysicalKey::Unidentified,
                        modifiers: modifiers | shift,
                        repeat: false,
                        phase: KeyPhase::Down,
                        text: None,
                    })),
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
