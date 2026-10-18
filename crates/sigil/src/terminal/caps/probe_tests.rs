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
    let reply = super::read_reply(&reader, |_| false);
    let open = late
        .join()
        .map_err(|_| std::io::Error::other("writer panicked"))??;
    let left = super::ready(&reader, std::time::Duration::ZERO).ok();
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

#[test]
fn a_truecolor_echo_and_a_version_reply_are_read() {
    use crate::terminal::caps::{Known, Version};
    let reply = parse("\x1bP1$r0;38:2::1:2:3m\x1b\\\x1bP>|WezTerm 20240203-110809\x1b\\\x1b[?62;c");
    let plain = parse("\x1bP0$r\x1b\\\x1b[?62;c");
    assert!(reply.truecolor);
    assert!(matches!(reply.version, Version::Named(Known::WezTerm)));
    assert!(!plain.truecolor);
    assert!(matches!(plain.version, Version::Silent));
}

#[test]
fn the_startup_probe_never_writes_an_image_query() {
    use crate::terminal::caps::{Question, Questions};
    let every = [
        Question::Sync,
        Question::Keys,
        Question::Color,
        Question::Version,
    ]
    .into_iter()
    .fold(Questions::default(), |ask, question| {
        ask.with(question, true)
    });
    assert!(!super::queries(every, true).contains("\x1b_G"));
}

fn every_question() -> super::Questions {
    use super::{Question, Questions};
    [
        Question::Sync,
        Question::Keys,
        Question::Color,
        Question::Version,
    ]
    .into_iter()
    .fold(Questions::default(), |ask, question| {
        ask.with(question, true)
    })
}

#[test]
fn a_silent_terminal_costs_one_deadline() -> std::io::Result<()> {
    use std::os::unix::net::UnixStream;
    let (input, _quiet) = UnixStream::pair()?;
    let mut sent = Vec::new();
    let started = std::time::Instant::now();
    let answers = super::ask_on(&input, &mut sent, every_question(), (true, true));
    let waited = started.elapsed();
    assert!(answers.is_none());
    assert!(
        waited < std::time::Duration::from_millis(800),
        "a silent terminal held startup for {waited:?}"
    );
    Ok(())
}

#[test]
fn every_startup_query_goes_out_in_one_exchange() -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    let (input, mut terminal) = UnixStream::pair()?;
    terminal.write_all(b"\x1b[?62;c")?;
    let mut sent = Vec::new();
    let _answers = super::ask_on(&input, &mut sent, every_question(), (true, true));
    let text = String::from_utf8_lossy(&sent);
    assert_eq!(text.matches("\x1b[?u").count(), 1, "{text:?}");
    assert_eq!(text.matches("\x1b[c").count(), 1, "{text:?}");
    assert!(text.ends_with("\x1b[c"), "{text:?}");
    assert!(text.contains("\x1b[6n"), "{text:?}");
    Ok(())
}

#[test]
fn a_reply_split_at_any_byte_reads_the_same() -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    let whole: &[u8] = b"\x1b[?2026;2$y\x1b[?1u\x1bP1$r38:2:1:2:3m\x1b\\\x1bP>|ghostty 1.2.0\x1b\\\x1b]11;rgb:0a0a/0a0a/0a0a\x1b\\\x1b[3;1R\x1b[3;3R\x1b[3;1R\x1b[3;1R\x1b[3;3R\x1b[?62;22c";
    let expected = parse(&String::from_utf8_lossy(whole));
    assert!(
        expected.sync && expected.kitty_keys && expected.truecolor,
        "{expected:?}"
    );
    assert!(
        matches!(expected.version, super::Version::Named(_)),
        "{expected:?}"
    );
    assert!(
        matches!(expected.measured, super::super::Measured::Columns(_)),
        "{expected:?}"
    );
    for split in 1..whole.len() {
        let (reader, mut writer) = UnixStream::pair()?;
        let (head, tail) = whole.split_at(split);
        writer.write_all(head)?;
        let late = std::thread::spawn({
            let tail = tail.to_vec();
            move || {
                std::thread::park_timeout(std::time::Duration::from_millis(2));
                writer.write_all(&tail).map(|()| writer)
            }
        });
        let reply = super::read_reply(&reader, |_| false);
        let _open = late
            .join()
            .map_err(|_| std::io::Error::other("writer panicked"))??;
        let got = reply.map(|reply| parse(&reply));
        assert_eq!(got.as_ref(), Some(&expected), "split at byte {split}");
    }
    Ok(())
}
