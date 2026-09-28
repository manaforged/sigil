use super::Glyphs;

#[test]
fn ascii_covers_every_family() {
    let table = [
        ("▏▎▍▌▋▊▉█", "|||#####"),
        ("░▒▓", ".:#"),
        ("▁▂▃▄▅▆▇", "_.-=+*#"),
        ("↑↓←→▲▼◀▶", "^v<>^v<>"),
        ("─│┌┐└┘├┤┬┴┼╭╮╰╯", "-|+++++++++++++"),
        ("·…°", "..o"),
        ("\u{2800}\u{2801}\u{28ff}", " .#"),
        ("ok", "ok"),
    ];
    for (text, ascii) in table {
        assert_eq!(Glyphs::Ascii.text(text), ascii, "{text}");
    }
}

#[test]
fn console_keeps_box_and_full_blocks() {
    let table = [
        ("─│┌┐└┘┼", "─│┌┐└┘┼"),
        ("╭╮╰╯", "┌┐└┘"),
        ("█░▒▓▀▄", "█░▒▓▀▄"),
        ("▏▌▉", "|██"),
        ("▁▃▇", "_▄█"),
        ("\u{28ff}", "#"),
    ];
    for (text, console) in table {
        assert_eq!(Glyphs::Console.text(text), console, "{text}");
    }
}

#[test]
fn unicode_is_identity() {
    assert_eq!(Glyphs::Unicode.text("▁▂⣿╭"), "▁▂⣿╭");
}
