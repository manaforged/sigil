#![allow(clippy::type_complexity)]

use std::io::Cursor;

use crate::kernel::ImageSource;

use super::{
    ImageFrame, ImagePainter, Mux, Placement, Protocol, base64, blocks, blocks_window, detect,
    detect_mux, png, png_window,
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

fn emit(
    painter: &mut ImagePainter,
    protocol: Protocol,
    next: &[Placement],
    rewritten: &[bool],
    redraw: bool,
) -> String {
    let mut out = Cursor::new(Vec::new());
    let frame = ImageFrame {
        next,
        rewritten,
        redraw,
    };
    painter
        .emit_mux(&mut out, protocol, None, frame)
        .expect("emit");
    String::from_utf8(out.into_inner()).expect("utf8")
}

#[test]
fn detection_prefers_the_override_then_known_terminals() {
    let cases: [(&[(&str, &str)], Option<Protocol>); 9] = [
        (
            &[
                ("SIGIL_GRAPHICS", "kitty"),
                ("TERM_PROGRAM", "Apple_Terminal"),
            ],
            Some(Protocol::Kitty),
        ),
        (
            &[("SIGIL_GRAPHICS", "none"), ("KITTY_WINDOW_ID", "1")],
            None,
        ),
        (
            &[("SIGIL_GRAPHICS", "bogus"), ("KITTY_WINDOW_ID", "1")],
            Some(Protocol::Kitty),
        ),
        (&[("TERM", "xterm-ghostty")], Some(Protocol::Kitty)),
        (&[("TERM_PROGRAM", "WezTerm")], Some(Protocol::Iterm2)),
        (
            &[("WEZTERM_EXECUTABLE", "/usr/bin/wezterm-gui")],
            Some(Protocol::Iterm2),
        ),
        (&[("TERM_PROGRAM", "iTerm.app")], Some(Protocol::Iterm2)),
        (&[("TERM_PROGRAM", "Apple_Terminal")], None),
        (
            &[("TMUX", "/tmp/x"), ("KITTY_WINDOW_ID", "1")],
            Some(Protocol::Kitty),
        ),
    ];
    for (pairs, want) in cases {
        assert_eq!(detect(env(pairs)), want, "{pairs:?}");
    }
    assert_eq!(
        detect_mux(env(&[("TMUX", "/tmp/x"), ("STY", "123")])),
        Some(Mux::Tmux)
    );
    assert_eq!(detect_mux(env(&[("STY", "123")])), Some(Mux::Screen));
    assert_eq!(detect_mux(env(&[("TERM", "xterm")])), None);
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

fn placed(
    at: (u16, u16),
    size: (u16, u16),
    source: ImageSource,
    crop: Option<(u32, u32)>,
) -> Placement {
    Placement {
        col: at.0,
        row: at.1,
        cols: size.0,
        rows: size.1,
        mark_col: 0,
        mark_row: 0,
        span_cols: size.0,
        span_rows: size.1,
        source,
        crop,
    }
}

#[test]
fn kitty_transmits_once_and_places_at_the_cell() {
    let place = placed((3, 4), (10, 5), source(2, 2), None);
    let quiet = [false; 20];
    let mut painter = ImagePainter::default();
    let first = emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&place),
        &quiet,
        false,
    );
    assert!(first.contains("\x1b_Ga=t,f=32,s=2,v=2,i="), "{first:?}");
    assert!(first.contains("U=1"), "{first:?}");
    assert!(first.contains(",c=10,r=5,q=2\x1b\\"), "{first:?}");
    assert!(
        !first.contains("\x1b[5;4H"),
        "a virtual placement is not a cursor overlay: {first:?}"
    );
    let unchanged = emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&place),
        &quiet,
        false,
    );
    assert!(unchanged.is_empty(), "an unchanged placement is not resent");
}

#[test]
fn kitty_keeps_a_virtual_placement_when_its_row_is_rewritten() {
    let place = placed((3, 4), (10, 5), source(2, 2), None);
    let quiet = [false; 20];
    let mut painter = ImagePainter::default();
    drop(emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&place),
        &quiet,
        false,
    ));
    let rewritten: Vec<bool> = (0..20).map(|row| row == 6).collect();
    let again = emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&place),
        &rewritten,
        false,
    );
    assert!(
        !again.contains("a=t,"),
        "data is transmitted once: {again:?}"
    );
    assert!(
        !again.contains("a=p,"),
        "placeholder cells carry the image, so a rewritten row does not re-place: {again:?}"
    );
    let gone = emit(&mut painter, Protocol::Kitty, &[], &quiet, false);
    assert!(gone.contains("\x1b_Ga=d,d=I,i="), "{gone:?}");
}

#[test]
fn kitty_scroll_keeps_the_full_placement_size() {
    let image = source(20, 16);
    let full = placed((0, 2), (10, 8), image.clone(), None);
    let mut painter = ImagePainter::default();
    let first = emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&full),
        &[false; 12],
        true,
    );
    assert!(first.contains(",c=10,r=8"), "{first:?}");
    let mut scrolled = full.clone();
    scrolled.row = 0;
    scrolled.rows = 3;
    scrolled.mark_row = 5;
    let again = emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&scrolled),
        &[false; 12],
        false,
    );
    assert!(
        !again.contains("a=p,"),
        "scrolling must not refit the image to the visible slice: {again:?}"
    );
    assert!(
        !again.contains("a=d,"),
        "the full placement stays while the image is partly off screen: {again:?}"
    );
    let mut grid = crate::terminal::paint::Grid::new(12, 8);
    grid.push_image(scrolled);
    grid.stamp_placeholders();
    let cell = grid.cell(0, 0).expect("visible origin");
    let extra = (super::kitty_id(&image) >> 24) as u8;
    let expected = crate::terminal::placeholder::grapheme(5, 0, extra).expect("mark");
    assert_eq!(
        cell.text.as_str(),
        expected,
        "a scrolled cell keeps its place in the full image"
    );
}

#[test]
fn kitty_placeholder_cells_carry_the_image() {
    let place = placed((1, 2), (2, 2), source(2, 2), None);
    let mut grid = crate::terminal::paint::Grid::new(8, 8);
    grid.push_image(place);
    grid.stamp_placeholders();
    let cell = grid.cell(1, 2).expect("origin");
    assert!(
        cell.text.starts_with('\u{10EEEE}'),
        "the image is a cell in the transcript, not an overlay: {:?}",
        cell.text
    );
    let image = grid.images().first().expect("placement");
    let id = super::kitty_id(&image.source);
    let crate::terminal::paint::Ink::Exact(colour) = cell.fg else {
        unreachable!("placeholder foreground is the image id");
    };
    assert_eq!(
        u32::from(colour.r) << 16 | u32::from(colour.g) << 8 | u32::from(colour.b),
        id & 0x00FF_FFFF
    );
}

#[test]
fn iterm_places_an_inline_png_sized_in_cells() {
    let place = placed((0, 1), (8, 4), source(3, 2), None);
    let mut painter = ImagePainter::default();
    let text = emit(
        &mut painter,
        Protocol::Iterm2,
        std::slice::from_ref(&place),
        &[false; 8],
        true,
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
    let place = placed((0, 0), (4, 1), source(4, 4), Some((2, 2)));
    let mut painter = ImagePainter::default();
    let kitty = emit(
        &mut painter,
        Protocol::Kitty,
        std::slice::from_ref(&place),
        &[false; 4],
        false,
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

#[test]
fn sigil_graphics_off_beats_a_named_terminal() {
    let env = |key: &str| match key {
        "SIGIL_GRAPHICS" => Some("off".to_string()),
        "KITTY_WINDOW_ID" => Some("1".to_string()),
        _ => None,
    };
    assert_eq!(detect(env), None);
}

#[test]
fn sixel_draws_at_the_measured_cell_size() {
    let source = ImageSource::from_rgba8(4, 4, vec![255; 64]).expect("image");
    let sized = super::sixel::encode(&source, (0, 4), (2, 1), Some([10, 20]));
    assert!(sized.starts_with("\x1bPq\"1;1;20;20"), "{sized:?}");
    let default = super::sixel::encode(&source, (0, 4), (2, 1), None);
    assert!(default.starts_with("\x1bPq\"1;1;16;16"), "{default:?}");
}
