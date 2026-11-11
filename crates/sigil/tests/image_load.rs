#![cfg(feature = "image-decode")]

use std::cell::Cell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_kittest::Harness;

const RED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/images/red-4x4.jpg"
);

#[test]
fn a_file_image_loads_off_the_ui_thread_and_shows_with_its_alt() {
    let first = Rc::new(Cell::new(None));
    let seen = Rc::clone(&first);
    let status = Rc::new(Cell::new(None));
    let export = Rc::clone(&status);
    let mut ui = Harness::new(move || {
        let load = load_image(RED).alt("Red square");
        seen.set(Some(load.status().peek().is_pending()));
        export.set(Some(load.status()));
        column().child(load)
    });
    ui.expect_text("Red square");
    assert_eq!(first.get(), Some(true), "the load did not start pending");
    let loaded = status.get().expect("status signal").peek();
    assert!(
        matches!(&loaded, Status::Ok(source) if (source.width(), source.height()) == (4, 4)),
        "expected a loaded 4x4 image, got {loaded:?}"
    );
}

#[test]
fn a_missing_file_shows_the_error_view_with_the_reason() {
    Harness::new(|| {
        column().child(
            load_image("no/such/picture.png")
                .alt("Picture")
                .error(|err| text(format!("Failed: {:?}", err.kind()))),
        )
    })
    .expect_text("Failed: Io");
}

#[test]
fn the_default_error_view_names_the_path() {
    Harness::new(|| column().child(load_image("file://no/such/logo.png"))).expect_text(
        "Could not load image: no/such/logo.png: No such file or directory (os error 2)",
    );
}

fn loaded_size(path: &str) -> Option<(u32, u32)> {
    let status = Rc::new(Cell::new(None));
    let export = Rc::clone(&status);
    let path = path.to_owned();
    let mut ui = Harness::new(move || {
        let load = load_image(path.clone()).alt("Picture");
        export.set(Some(load.status()));
        column().child(load)
    });
    ui.expect_text("Picture");
    match status.get().expect("status signal").peek() {
        Status::Ok(source) => Some((source.width(), source.height())),
        _ => None,
    }
}

#[test]
fn a_changed_file_loads_again() {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("image-load-changed.png");
    image::RgbaImage::new(4, 4).save(&path).expect("write 4x4");
    let path_text = path.to_str().expect("utf-8 path").to_owned();
    assert_eq!(loaded_size(&path_text), Some((4, 4)));
    image::RgbaImage::new(2, 2).save(&path).expect("write 2x2");
    assert_eq!(loaded_size(&path_text), Some((2, 2)));
}
