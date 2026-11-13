#![cfg(feature = "image-decode")]

use std::cell::Cell;
use std::rc::Rc;

use sigil::kernel::Role;
use sigil::node::ObjectFit;
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
    let reason = std::fs::read("no/such/logo.png").expect_err("the file is missing");
    Harness::new(|| column().child(load_image("no/such/logo.png")))
        .expect_text(&format!("Could not load image: no/such/logo.png: {reason}"));
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

#[cfg(feature = "image-http")]
fn serve(status: &'static str, body: Vec<u8>) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let url = format!(
        "http://{}/picture.png",
        listener.local_addr().expect("local address")
    );
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept the request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !request.ends_with(b"\r\n\r\n") {
            let read = stream.read(&mut chunk).expect("read the request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(chunk.get(..read).unwrap_or_default());
        }
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).expect("write the head");
        stream.write_all(&body).expect("write the body");
    });
    url
}

#[cfg(feature = "image-http")]
#[test]
fn an_http_image_loads_off_the_ui_thread() {
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::new(3, 5)
        .write_to(&mut png, image::ImageFormat::Png)
        .expect("encode a png");
    let url = serve("200 OK", png.into_inner());
    assert_eq!(loaded_size(&url), Some((3, 5)));
}

#[cfg(feature = "image-http")]
#[test]
fn an_http_error_status_shows_the_error_view() {
    let url = serve("404 Not Found", Vec::new());
    Harness::new(move || {
        column().child(
            load_image(url.clone())
                .alt("Picture")
                .error(|err| text(format!("Failed: {:?}", err.kind()))),
        )
    })
    .expect_text("Failed: Http");
}

fn file_url(host: &str) -> String {
    let path = RED.replace('\\', "/");
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    format!("file://{host}{path}")
}

#[test]
fn file_urls_with_an_empty_or_localhost_host_load() {
    for url in [file_url(""), file_url("localhost")] {
        assert_eq!(loaded_size(&url), Some((4, 4)), "{url}");
    }
}

#[test]
fn a_sized_load_image_fills_its_box() {
    let mut ui = Harness::new(|| {
        column().child(
            load_image(RED)
                .alt("Red square")
                .fit(ObjectFit::Contain)
                .width(40.0)
                .height(24.0),
        )
    })
    .pixels(200, 120);
    ui.expect_text("Red square");
    let bounds = ui
        .get((Role::Image, "Red square"))
        .bounds()
        .expect("image bounds");
    assert_eq!((bounds.width, bounds.height), (40.0, 24.0));
}
