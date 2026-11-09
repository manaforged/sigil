use std::time::Instant;

use crate::events::Event;
use crate::profile;
use crate::terminal::screen::Screen;
use crate::terminal::term::Term;
use crate::{AppConfig, AppResult, Error, Host};

use super::{Hooks, isolated, recover, tick};

pub(super) struct Turn<'a> {
    last: &'a mut Instant,
    screen: &'a mut Screen,
    host: &'a mut Host,
    term: &'a mut Term,
}

impl<'a> Turn<'a> {
    pub(super) const fn new(
        last: &'a mut Instant,
        screen: &'a mut Screen,
        host: &'a mut Host,
        term: &'a mut Term,
    ) -> Self {
        Self {
            last,
            screen,
            host,
            term,
        }
    }

    pub(super) fn take(
        &mut self,
        config: &AppConfig,
        incoming: Vec<Event>,
        (cols, rows): (u16, u16),
        hooks: &mut Hooks<'_>,
    ) -> AppResult<bool> {
        let size = match hooks.region_rows.as_mut() {
            Some(hook) if config.inline => (cols, hook().clamp(1, rows.max(1))),
            _ => (cols, rows),
        };
        self.screen.set_dt(tick(self.last));
        profile::begin_frame(profile::BackendStatus::TERMINAL);
        let (screen, host) = (&mut *self.screen, &mut *self.host);
        let frame = isolated(config.panic_policy, || {
            screen.frame(host, incoming, size).map(|_| ())
        })?;
        if frame.is_none() {
            recover(self.term)?;
        }
        if let Some(after) = hooks.after_frame.as_mut() {
            after(self.host, self.screen);
        }
        for message in self.screen.take_above() {
            if let Err(err) = crate::terminal::print_above(&message) {
                tracing::warn!(target: "sigil::terminal", error = %err, "announcement write failed");
            }
        }
        {
            let _phase = profile::span(crate::profile::Phase::TuiPresent);
            self.term.paint(self.screen).map_err(Error::io)?;
        }
        profile::tick_frame();
        Ok(self.screen.exited())
    }
}
