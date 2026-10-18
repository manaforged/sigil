use std::io::{self, Write};

use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};

use super::LIVE_ROWS;
use crate::terminal::screen::Screen;

pub(crate) fn paint_frame(
    out: &mut impl Write,
    screen: &mut Screen,
    redraw: bool,
    inline: bool,
) -> io::Result<()> {
    let painted = write_frame(out, screen, redraw, inline);
    if painted.is_err() {
        screen.keep_pending();
    }
    painted
}

fn write_frame(
    out: &mut impl Write,
    screen: &mut Screen,
    redraw: bool,
    inline: bool,
) -> io::Result<()> {
    if !redraw && !screen.needs_flush() {
        return Ok(());
    }
    let sync = screen.caps().sync;
    if sync {
        queue!(out, Print("\x1b[?2026h"))?;
    }
    if redraw && !inline {
        queue!(out, Clear(ClearType::All))?;
    }
    if redraw {
        screen.invalidate();
    }
    if inline {
        screen.flush_inline(out)?;
        LIVE_ROWS.with(|rows| rows.set(u16::try_from(screen.rows()).unwrap_or(u16::MAX)));
    } else {
        screen.flush(out)?;
    }
    if sync {
        queue!(out, Print("\x1b[?2026l"))?;
    }
    out.flush()
}
