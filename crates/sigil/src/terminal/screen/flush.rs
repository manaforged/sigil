use std::io::{self, Write};

use super::*;

impl Screen {
    /// Writes the changed cells to `out`.
    pub fn flush(&mut self, out: &mut impl Write) -> Result<(), crate::Error> {
        self.measure(out, false)
    }

    /// Writes the changed cells to `out` in inline mode.
    pub fn flush_inline(&mut self, out: &mut impl Write) -> Result<(), crate::Error> {
        self.measure(out, true)
    }

    fn measure(&mut self, out: &mut impl Write, inline: bool) -> Result<(), crate::Error> {
        if !profile::enabled() {
            return self.emit(out, inline).map_err(Into::into);
        }
        let _phase = profile::span(profile::Phase::TuiWrite);
        let mut measured = CountingWriter::new(out);
        let result = self.emit(&mut measured, inline);
        profile::count(profile::Counter::TuiBytes, measured.bytes);
        profile::count(profile::Counter::TuiWrites, measured.writes);
        result.map_err(Into::into)
    }

    fn emit(&mut self, out: &mut impl Write, inline: bool) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.dirty = false;
        let written = if inline {
            self.write_inline(out)
        } else {
            self.write(out)
        };
        if written.is_err() {
            self.fresh = true;
            self.dirty = true;
        }
        written
    }

    fn write_inline(&mut self, out: &mut impl Write) -> io::Result<()> {
        let prev = (!self.fresh).then_some(&self.prev);
        crate::terminal::write::inline(
            out,
            prev,
            &self.grid,
            (self.live_rows, self.announcer.caret),
            &self.caps,
        )?;
        self.live_rows = self.grid.rows();
        self.fresh = false;
        self.stale = None;
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    fn write(&mut self, out: &mut impl Write) -> io::Result<()> {
        let kitty = self.caps.graphics == Some(crate::terminal::graphics::Protocol::Kitty)
            && self.caps.color == crate::terminal::caps::ColorDepth::Truecolor;
        if kitty {
            self.grid.stamp_placeholders();
        }
        let full = self.fresh
            || self.prev.cols() != self.grid.cols()
            || self.prev.rows() != self.grid.rows();
        let tracked = if self.caps.graphics.is_some() || profile::enabled() {
            self.grid.rows()
        } else {
            0
        };
        let rewritten = self.rewritten(tracked, full);
        if kitty {
            self.emit_images(out, &rewritten, full)?;
        }
        profile::count(
            profile::Counter::TuiRowsWritten,
            rewritten.iter().filter(|row| **row).count() as u64,
        );
        if full {
            self.fresh = false;
            self.stale = None;
            self.prev.fit(&self.grid);
            crate::terminal::write::all(out, &self.grid, &self.caps)?;
        } else {
            crate::terminal::write::dirty(
                out,
                &self.prev,
                &self.grid,
                &self.caps,
                self.damage.as_ref(),
            )?;
        }
        if !kitty {
            self.emit_images(out, &rewritten, full)?;
        }
        crate::terminal::write::cursor(out, self.grid.cursor(), self.announcer.caret, &self.caps)?;
        core::mem::swap(&mut self.prev, &mut self.grid);
        Ok(())
    }

    fn emit_images(
        &mut self,
        out: &mut impl Write,
        rewritten: &[bool],
        redraw: bool,
    ) -> io::Result<()> {
        self.painter.emit(
            out,
            self.caps,
            crate::terminal::graphics::ImageFrame {
                next: self.grid.images(),
                rewritten,
                redraw,
            },
        )
    }

    fn rewritten(&self, tracked: usize, full: bool) -> Vec<bool> {
        let differs = |row: usize| {
            !crate::terminal::write::same_row(self.prev.row(row), self.grid.row(row), &self.caps)
        };
        self.damage.as_ref().filter(|_| !full).map_or_else(
            || (0..tracked).map(|row| full || differs(row)).collect(),
            |damage| {
                let mut rewritten = vec![false; tracked];
                for row in damage.rows(tracked) {
                    if let Some(slot) = rewritten.get_mut(row) {
                        *slot = differs(row);
                    }
                }
                rewritten
            },
        )
    }
}
