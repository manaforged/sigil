use super::Known;

#[derive(Clone, Copy, Default)]
pub(super) enum Version {
    #[default]
    Silent,
    Unnamed,
    Named(Known),
}

#[derive(Clone, Copy)]
pub(super) enum Question {
    Graphics,
    Sync,
    Keys,
    Color,
    Version,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Questions(u8);

impl Questions {
    pub(super) const fn with(self, question: Question, open: bool) -> Self {
        if open {
            Self(self.0 | 1 << question as u8)
        } else {
            self
        }
    }

    pub(super) const fn none(self) -> bool {
        self.0 == 0
    }

    pub(super) const fn has(self, question: Question) -> bool {
        self.0 & (1 << question as u8) != 0
    }
}
