use std::convert::Infallible;
use std::io;

use crate::events::Event;
use crate::terminal::graphics::Mux;
use crate::terminal::term::Term;
use crate::{AppResult, Host};

/// The pixel host of a build without the renderer: it has no value, so pixel mode never mounts.
pub(super) struct Draw(Infallible);

impl Draw {
    pub(super) const fn host(&self) -> &Host {
        match self.0 {}
    }

    pub(super) const fn enter(&self) -> io::Result<()> {
        match self.0 {}
    }

    pub(super) const fn leave(&self, _: Option<Mux>) -> io::Result<()> {
        match self.0 {}
    }

    pub(super) const fn turn(
        &self,
        _: (Vec<Event>, f32),
        _: (u16, u16),
        _: [u16; 2],
        _: &Term,
    ) -> AppResult<bool> {
        match self.0 {}
    }
}
