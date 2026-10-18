use std::io::Write;

use sigil::Host;
use sigil::events::EventRouter;
use sigil::reactive::{Runtime, signal};
use sigil::terminal::Screen;

use super::live::seen;
use super::oracle::{COLS, Model, ROWS, tree};

struct Cut {
    left: usize,
    step: usize,
    bytes: Vec<u8>,
}

impl Cut {
    const fn open() -> Self {
        Self::after(usize::MAX, usize::MAX)
    }

    const fn after(left: usize, step: usize) -> Self {
        Self {
            left,
            step,
            bytes: Vec::new(),
        }
    }
}

impl Write for Cut {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.left == 0 {
            return Err(std::io::Error::other("the terminal went away"));
        }
        let taken = buf.len().min(self.left).min(self.step);
        self.bytes
            .extend_from_slice(buf.get(..taken).unwrap_or_default());
        self.left -= taken;
        Ok(taken)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn lines(edits: &[(usize, &str)]) -> Model {
    let mut model = Model {
        lines: (0..ROWS).map(|row| format!("row {row} text")).collect(),
        ..Model::default()
    };
    for (at, word) in edits {
        model.lines[*at] = (*word).to_string();
    }
    model
}

const FIRST: (usize, &str) = (1, "first change");
const SECOND: (usize, &str) = (9, "second change");

fn replayed(cut: Cut) -> (String, usize) {
    Runtime::scope("faults-replayed", || {
        let state = signal(lines(&[]));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        let mut term = vt100::Parser::new(ROWS, COLS, 0);
        let mut flush =
            |view: &mut Screen, embed: &mut Host, route: &mut EventRouter, mut out: Cut| {
                view.frame(embed, route, [], (COLS, ROWS)).expect("frame");
                let failed = view.flush(&mut out).is_err();
                term.process(&out.bytes);
                (out.bytes.len(), failed)
            };
        for _ in 0..2 {
            flush(&mut screen, &mut host, &mut router, Cut::open());
        }
        state.set(lines(&[FIRST]));
        let (written, _) = flush(&mut screen, &mut host, &mut router, cut);
        state.set(lines(&[FIRST, SECOND]));
        flush(&mut screen, &mut host, &mut router, Cut::open());
        (seen(term.screen(), (COLS, ROWS)), written)
    })
}

fn expected() -> String {
    fresh(&[FIRST, SECOND])
}

fn fresh(edits: &[(usize, &str)]) -> String {
    Runtime::scope("faults-fresh", || {
        let state = signal(lines(edits));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        let mut out = Vec::new();
        screen.flush(&mut out).expect("flush");
        let mut term = vt100::Parser::new(ROWS, COLS, 0);
        term.process(&out);
        seen(term.screen(), (COLS, ROWS))
    })
}

#[test]
fn a_write_cut_at_any_byte_recovers_on_the_next_flush() {
    let fresh = expected();
    let (whole, total) = replayed(Cut::open());
    assert_eq!(whole, fresh, "an uncut run differs from a fresh render");
    assert!(total > 40, "the cut frame wrote only {total} bytes");
    for cut in 0..total {
        let (shown, _) = replayed(Cut::after(cut, usize::MAX));
        assert_eq!(shown, fresh, "a write cut after {cut} of {total} bytes");
    }
}

#[test]
fn one_byte_writes_show_the_same_screen() {
    let (shown, _) = replayed(Cut::after(usize::MAX, 1));
    assert_eq!(shown, expected());
}

#[test]
fn a_failed_flush_is_written_by_the_next_flush_alone() {
    let (shown, written) = Runtime::scope("faults-retry", || {
        let state = signal(lines(&[]));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        let mut term = vt100::Parser::new(ROWS, COLS, 0);
        for _ in 0..2 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            let mut out = Vec::new();
            screen.flush(&mut out).expect("flush");
            term.process(&out);
        }
        state.set(lines(&[FIRST]));
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        let mut cut = Cut::after(10, usize::MAX);
        assert!(screen.flush(&mut cut).is_err());
        term.process(&cut.bytes);
        let mut out = Vec::new();
        screen.flush(&mut out).expect("flush");
        term.process(&out);
        (seen(term.screen(), (COLS, ROWS)), out.len())
    });
    assert!(written > 0, "the retry wrote nothing");
    assert_eq!(shown, fresh(&[FIRST]));
}
