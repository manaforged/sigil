use super::Host;
use crate::app::config::QuitKeys;

impl Host {
    pub(crate) const fn set_quit(&mut self, quit: QuitKeys) {
        self.router.set_quit(quit);
    }
}
