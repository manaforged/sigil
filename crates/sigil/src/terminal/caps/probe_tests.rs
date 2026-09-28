use super::parse;
use crate::terminal::caps::Backdrop;

#[test]
fn an_osc_11_reply_sets_the_backdrop_from_its_luma() {
    let dark = parse("\x1b]11;rgb:0a0a/0a0a/0a0a\x1b\\\x1b[?62;c");
    let light = parse("\x1b]11;rgb:ffff/ffff/f0f0\x07\x1b[?62;c");
    let none = parse("\x1b[?62;c");
    assert_eq!(dark.backdrop, Some(Backdrop::Dark));
    assert_eq!(light.backdrop, Some(Backdrop::Light));
    assert_eq!(none.backdrop, None);
}

#[test]
fn a_late_reply_is_read_to_its_end_before_input_starts() -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    let (reader, mut writer) = UnixStream::pair()?;
    let late = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        writer
            .write_all(b"\x1b]11;rgb:0a0a/0a0a/0a0a\x1b\\\x1b[?2026;2$y\x1b[?62;c")
            .map(|()| writer)
    });
    let reply = super::read_reply(&reader);
    let open = late
        .join()
        .map_err(|_| std::io::Error::other("writer panicked"))??;
    let left = super::ready(&reader, std::time::Duration::ZERO);
    drop(open);
    assert_eq!(
        reply.map(|reply| parse(&reply).backdrop),
        Some(Some(Backdrop::Dark))
    );
    assert_eq!(
        left,
        Some(false),
        "no reply byte may be left for the input parser"
    );
    Ok(())
}
