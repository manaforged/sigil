use super::{Caps, ColorDepth, Erase, Fills, Keyboard};
use crate::app::Ambiguous;
use crate::terminal::glyphs::Glyphs;

fn caps(pairs: &[(&str, &str)]) -> Caps {
    Caps::detect(|key| {
        pairs
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| (*value).to_string())
    })
}

type Row = (&'static [(&'static str, &'static str)], ColorDepth);

#[test]
fn term_selects_colour_depth() {
    let table: [Row; 13] = [
        (&[("TERM", "xterm-ghostty")], ColorDepth::Truecolor),
        (&[("TERM", "xterm-kitty")], ColorDepth::Truecolor),
        (&[("TERM", "wezterm")], ColorDepth::Truecolor),
        (&[("TERM", "alacritty")], ColorDepth::Truecolor),
        (&[("TERM", "foot")], ColorDepth::Truecolor),
        (&[("TERM", "contour")], ColorDepth::Truecolor),
        (&[("TERM", "xterm-direct")], ColorDepth::Truecolor),
        (
            &[("TERM", "xterm"), ("TERM_PROGRAM", "iTerm.app")],
            ColorDepth::Truecolor,
        ),
        (
            &[("TERM", "xterm"), ("COLORTERM", "24bit")],
            ColorDepth::Truecolor,
        ),
        (
            &[
                ("TERM", "xterm-256color"),
                ("TERM_PROGRAM", "Apple_Terminal"),
            ],
            ColorDepth::Ansi256,
        ),
        (&[("TERM", "xterm")], ColorDepth::Ansi16),
        (
            &[("TERM", "xterm"), ("SIGIL_COLOR", "256")],
            ColorDepth::Ansi256,
        ),
        (
            &[("TERM", "xterm-kitty"), ("SIGIL_COLOR", "none")],
            ColorDepth::Monochrome,
        ),
    ];
    for (env, depth) in table {
        assert_eq!(caps(env).color, depth, "{env:?}");
    }
}

#[test]
fn mux_ignores_inherited_outer_terminal() {
    let inside = caps(&[
        ("TERM", "tmux-256color"),
        ("TMUX", "/tmp/tmux-1/default,1,0"),
        ("KITTY_WINDOW_ID", "1"),
        ("GHOSTTY_RESOURCES_DIR", "/usr/share/ghostty"),
        ("WEZTERM_EXECUTABLE", "/usr/bin/wezterm"),
        ("LC_TERMINAL", "iTerm2"),
        ("VTE_VERSION", "7600"),
        ("WT_SESSION", "1"),
    ]);
    assert_eq!(inside.color, ColorDepth::Ansi256);
    assert_eq!(inside.keyboard, Keyboard::Legacy);
    assert!(!inside.hyperlinks);
    let outside = caps(&[("TERM", "xterm-256color"), ("KITTY_WINDOW_ID", "1")]);
    assert_eq!(outside.color, ColorDepth::Truecolor);
    assert_eq!(outside.keyboard, Keyboard::Kitty);
    let unknown = caps(&[("TERM", "xterm-256color")]);
    assert_eq!(unknown.keyboard, Keyboard::Legacy);
}

#[test]
fn glyphs_follow_term_locale_and_override() {
    assert_eq!(caps(&[("TERM", "xterm-256color")]).glyphs, Glyphs::Unicode);
    assert_eq!(caps(&[("TERM", "linux")]).glyphs, Glyphs::Console);
    assert_eq!(caps(&[("TERM", "vt100")]).glyphs, Glyphs::Ascii);
    assert_eq!(
        caps(&[("TERM", "xterm"), ("LANG", "C")]).glyphs,
        Glyphs::Ascii
    );
    assert_eq!(
        caps(&[("TERM", "xterm"), ("SIGIL_ASCII", "1")]).glyphs,
        Glyphs::Ascii
    );
}

#[test]
fn ambiguous_width_and_bce_come_from_env() {
    assert_eq!(caps(&[("TERM", "xterm")]).ambiguous, Ambiguous::Narrow);
    assert_eq!(
        caps(&[("TERM", "xterm"), ("SIGIL_AMBIGUOUS_WIDE", "1")]).ambiguous,
        Ambiguous::Wide
    );
    assert_eq!(caps(&[("TERM", "screen-256color")]).erase, Erase::Default);
    assert_eq!(caps(&[("TERM", "xterm")]).erase, Erase::Background);
}

#[cfg(unix)]
#[test]
fn probe_replies_raise_colour_links_and_notices_behind_a_multiplexer() {
    use super::Known;
    use super::{Answers, Version, draft};
    let env = |key: &str| match key {
        "TERM" => Some("screen-256color".to_owned()),
        "TMUX" => Some("/tmp/tmux-1/default,1,0".to_owned()),
        _ => None,
    };
    let before = draft(&env);
    assert_eq!(before.caps.color, ColorDepth::Ansi256);
    assert!(!before.caps.hyperlinks);
    let answers = Answers {
        truecolor: true,
        version: Version::Named(Known::WezTerm),
        ..Answers::default()
    };
    let after = draft(&env).settle(&answers);
    assert_eq!(after.color, ColorDepth::Truecolor);
    assert!(after.hyperlinks);
    assert!(after.notices.progress && after.notices.notify.is_some());
}

#[cfg(unix)]
#[test]
fn a_query_answer_never_turns_on_images() {
    let env = |key: &str| (key == "TERM").then(|| "xterm-256color".to_string());
    let answers = super::probe::parse("\x1b_Gi=31;OK\x1b\\\x1b[?62;4;22c");
    let caps = super::draft(&env).settle(&answers);
    assert_eq!(caps.graphics, None);
}

#[test]
fn terminal_app_fills_whole_cells_even_behind_a_multiplexer() {
    let fills = |vars: &[(&str, &str)]| caps(vars).fills;
    assert_eq!(fills(&[("TERM_PROGRAM", "Apple_Terminal")]), Fills::Whole);
    assert_eq!(
        fills(&[
            ("TERM_PROGRAM", "herdr"),
            ("__CFBundleIdentifier", "com.apple.Terminal")
        ]),
        Fills::Whole
    );
    assert_eq!(fills(&[("TERM_PROGRAM", "ghostty")]), Fills::Partial);
    assert_eq!(fills(&[]), Fills::Partial);
}

#[test]
fn sigil_fills_overrides_the_detected_fills() {
    let fills = |vars: &[(&str, &str)]| caps(vars).fills;
    assert_eq!(
        fills(&[
            ("TERM_PROGRAM", "Apple_Terminal"),
            ("SIGIL_FILLS", "partial")
        ]),
        Fills::Partial
    );
    assert_eq!(
        fills(&[("TERM_PROGRAM", "ghostty"), ("SIGIL_FILLS", "whole")]),
        Fills::Whole
    );
}
