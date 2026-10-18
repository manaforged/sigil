use crossterm::terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled};

pub(super) struct Raw {
    restore: bool,
}

impl Raw {
    pub(super) fn enter() -> std::io::Result<Self> {
        let restore = !is_raw_mode_enabled().unwrap_or(true);
        if restore {
            enable_raw_mode()?;
        }
        Ok(Self { restore })
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        if self.restore {
            drop(disable_raw_mode());
        }
    }
}
