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
