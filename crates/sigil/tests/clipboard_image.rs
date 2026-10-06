#![cfg(feature = "clipboard-image")]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::clipboard::{ClipError, Clipboard, Delivery};
use sigil::kernel::ImageSource;

struct Recorder(Rc<RefCell<Vec<(u32, u32)>>>);

impl Clipboard for Recorder {
    fn set(&self, _text: &str) -> Result<Delivery, ClipError> {
        Ok(Delivery::Confirmed)
    }

    fn get(&self) -> Option<String> {
        None
    }

    fn set_image(&self, image: &ImageSource) -> Result<Delivery, ClipError> {
        self.0.borrow_mut().push((image.width(), image.height()));
        Ok(Delivery::Confirmed)
    }
}

#[test]
fn copy_image_writes_the_image_to_the_installed_clipboard() {
    let written = Rc::new(RefCell::new(Vec::new()));
    sigil::clipboard::install(Recorder(Rc::clone(&written)));
    let image = ImageSource::from_rgba8(2, 3, vec![255; 2 * 3 * 4]).expect("image");
    assert_eq!(
        sigil::clipboard::copy_image(&image),
        Ok(Delivery::Confirmed)
    );
    assert_eq!(*written.borrow(), vec![(2, 3)]);
}
