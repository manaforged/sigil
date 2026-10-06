use alloc::string::String;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasteEvent {
    pub text: String,
    propagation_stopped: bool,
}

impl PasteEvent {
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self {
            text,
            propagation_stopped: false,
        }
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}
