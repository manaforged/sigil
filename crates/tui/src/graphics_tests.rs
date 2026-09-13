use std::io::Cursor;

use sigil::kernel::ImageSource;

use super::{
    Frame, Painter, Placement, Protocol, base64, blocks, blocks_window, detect, png, png_window,
};

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |key| {
        pairs
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| (*value).to_string())
    }
}

fn source(width: u32, height: u32) -> ImageSource {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[
                u8::try_from(x * 40).unwrap_or(u8::MAX),
                u8::try_from(y * 40).unwrap_or(u8::MAX),
                7,
                255,
            ]);
        }
    }
    ImageSource::from_rgba8(width, height, rgba).expect("source")
}

fn emit(painter: &mut Painter, protocol: Protocol, frame: Frame<'_>) -> String {
    let mut out = Cursor::new(Vec::new());
    painter.emit_with(&mut out, protocol, frame).expect("emit");
    String::from_utf8(out.into_inner()).expect("utf8")
}

#[test]
fn detection_prefers_the_override_then_known_terminals() {
    assert_eq!(
        detect(env(&[
            ("SIGIL_GRAPHICS", "kitty"),
            ("TERM_PROGRAM", "Apple_Terminal")
        ])),
        Some(Protocol::Kitty)
    );
    assert_eq!(
        detect(env(&[("SIGIL_GRAPHICS", "none"), ("KITTY_WINDOW_ID", "1")])),
        None
    );
    assert_eq!(
        detect(env(&[("TERM", "xterm-ghostty")])),
        Some(Protocol::Kitty)
    );
    assert_eq!(
        detect(env(&[("TERM_PROGRAM", "WezTerm")])),
        Some(Protocol::Kitty)
    );
    assert_eq!(
        detect(env(&[("TERM_PROGRAM", "iTerm.app")])),
        Some(Protocol::Iterm2)
    );
    assert_eq!(detect(env(&[("TERM_PROGRAM", "Apple_Terminal")])), None);
    assert_eq!(
        detect(env(&[("TMUX", "/tmp/x"), ("KITTY_WINDOW_ID", "1")])),
        None
    );
}

#[test]
fn base64_matches_the_standard_alphabet_and_padding() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn png_round_trips_through_a_decoder() {
    let image = source(5, 3);
    let bytes = png(&image);
    let decoded = ::image::load_from_memory(&bytes)
        .expect("decodable png")
        .into_rgba8();
    assert_eq!(decoded.dimensions(), (5, 3));
    assert_eq!(decoded.as_raw().as_slice(), image.rgba());
}

fn placed() -> Placement {
    Placement {
        col: 3,
        row: 4,
        cols: 10,
        rows: 5,
        source: source(2, 2),
        crop: None,
    }
}

#[test]
fn kitty_transmits_once_and_places_at_the_cell() {
    let place = placed();
    let quiet = [false; 20];
    let mut painter = Painter::default();
    let first = emit(
        &mut painter,
        Protocol::Kitty,
        Frame {
            next: std::slice::from_ref(&place),
            rewritten: &quiet,
            redraw: false,
        },
    );
    assert!(first.contains("\x1b_Ga=t,f=32,s=2,v=2,i="), "{first:?}");
    assert!(first.contains(",c=10,r=5,C=1,q=2\x1b\\"), "{first:?}");
    assert!(
        first.contains("\x1b[5;4H"),
        "cursor moves to row 4 col 3: {first:?}"
    );
    let unchanged = emit(
        &mut painter,
        Protocol::Kitty,
        Frame {
            next: std::slice::from_ref(&place),
            rewritten: &quiet,
            redraw: false,
        },
    );
    assert!(unchanged.is_empty(), "an unchanged placement is not resent");
}

#[test]
fn kitty_replaces_after_a_touched_row_and_deletes_when_gone() {
    let place = placed();
    let quiet = [false; 20];
    let mut painter = Painter::default();
    drop(emit(
        &mut painter,
        Protocol::Kitty,
        Frame {
            next: std::slice::from_ref(&place),
            rewritten: &quiet,
            redraw: false,
        },
    ));
    let rewritten: Vec<bool> = (0..20).map(|row| row == 6).collect();
    let again = emit(
        &mut painter,
        Protocol::Kitty,
        Frame {
            next: std::slice::from_ref(&place),
            rewritten: &rewritten,
            redraw: false,
        },
    );
    assert!(
        !again.contains("a=t,"),
        "data is transmitted once: {again:?}"
    );
    assert!(
        again.contains("a=p,"),
        "a touched row places again: {again:?}"
    );
    let gone = emit(
        &mut painter,
        Protocol::Kitty,
        Frame {
            next: &[],
            rewritten: &quiet,
            redraw: false,
        },
    );
    assert!(gone.contains("\x1b_Ga=d,d=i,i="), "{gone:?}");
}

#[test]
fn iterm_places_an_inline_png_sized_in_cells() {
    let place = Placement {
        col: 0,
        row: 1,
        cols: 8,
        rows: 4,
        source: source(3, 2),
        crop: None,
    };
    let mut painter = Painter::default();
    let text = emit(
        &mut painter,
        Protocol::Iterm2,
        Frame {
            next: std::slice::from_ref(&place),
            rewritten: &[false; 8],
            redraw: true,
        },
    );
    assert!(
        text.contains("\x1b]1337;File=inline=1;width=8;height=4;preserveAspectRatio=0:iVBOR"),
        "{text:?}"
    );
    assert!(text.ends_with('\x07'), "{text:?}");
}

#[test]
fn blocks_sample_top_and_bottom_pixels_per_cell() {
    let image = source(4, 4);
    let cells = blocks(&image, 2, 2);
    assert_eq!(cells.len(), 2);
    let at = |row: usize, col: usize| cells.get(row).and_then(|line| line.get(col)).copied();
    let (top, bottom) = at(0, 0).expect("cell");
    assert_eq!((top.r, top.g), (0, 0));
    assert_eq!((bottom.r, bottom.g), (0, 40));
    let (later, _) = at(1, 1).expect("cell");
    assert_eq!((later.r, later.g), (80, 80));
}

#[test]
fn a_crop_selects_source_rows_for_every_protocol() {
    let place = Placement {
        col: 0,
        row: 0,
        cols: 4,
        rows: 1,
        source: source(4, 4),
        crop: Some((2, 2)),
    };
    let mut painter = Painter::default();
    let kitty = emit(
        &mut painter,
        Protocol::Kitty,
        Frame {
            next: std::slice::from_ref(&place),
            rewritten: &[false; 4],
            redraw: false,
        },
    );
    assert!(kitty.contains(",x=0,y=2,w=4,h=2,"), "{kitty:?}");
    let bytes = png_window(&place.source, 2, 2);
    let decoded = ::image::load_from_memory(&bytes).expect("png").into_rgba8();
    assert_eq!(decoded.dimensions(), (4, 2));
    assert_eq!(decoded.get_pixel(0, 0).0, [0, 80, 7, 255]);
    let cells = blocks_window(&place.source, (2, 2), 4, 1);
    let (top, bottom) = cells
        .first()
        .and_then(|line| line.first())
        .copied()
        .expect("cell");
    assert_eq!((top.g, bottom.g), (80, 120));
}
