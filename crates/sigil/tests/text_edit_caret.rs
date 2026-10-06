#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::prelude::*;

#[test]
fn caret_rect_follows_the_caret() {
    use sigil::geometry::Rect;
    use sigil_kittest::Harness;
    type CaretSlot = Rc<RefCell<Option<Signal<Option<Rect>>>>>;
    let rect: CaretSlot = Rc::new(RefCell::new(None));
    let slot = Rc::clone(&rect);
    let mut ui = Harness::new(move || {
        let caret = signal(None::<Rect>);
        *slot.borrow_mut() = Some(caret);
        text_edit(signal(String::new()))
            .autofocus()
            .caret_rect(caret)
    })
    .size(40, 6);
    ui.settle();
    let caret = rect.borrow().expect("caret signal");
    let start = caret.peek().expect("caret rect after focus").origin.x;
    ui.type_text("abc");
    ui.settle();
    let after = caret.peek().expect("caret rect after typing").origin.x;
    assert!(after > start, "caret x stayed at {start} after typing");
}
