use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipError(String);

impl ClipError {
    pub fn new(msg: impl fmt::Display) -> Self {
        Self(msg.to_string())
    }
}

impl fmt::Display for ClipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ClipError {}

pub trait Clipboard {
    fn set(&self, text: &str) -> Result<(), ClipError>;
    fn get(&self) -> Option<String>;
}

pub struct NoopClipboard;

impl Clipboard for NoopClipboard {
    fn set(&self, _text: &str) -> Result<(), ClipError> {
        Err(ClipError::new(
            "no clipboard handle is installed; call install first",
        ))
    }
    fn get(&self) -> Option<String> {
        None
    }
}

thread_local! {
    static CURRENT: RefCell<Rc<dyn Clipboard>> = RefCell::new(Rc::new(NoopClipboard));
}

pub fn install(clip: impl Clipboard + 'static) {
    CURRENT.with(|slot| *slot.borrow_mut() = Rc::new(clip));
}

#[must_use]
pub fn clipboard() -> Rc<dyn Clipboard> {
    CURRENT.with(|slot| Rc::clone(&*slot.borrow()))
}

pub(crate) fn write(clip: &dyn Clipboard, text: &str) -> Result<(), ClipError> {
    match clip.set(text) {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::warn!(target: "sigil::clipboard", error = %err, "clipboard write failed");
            Err(err)
        }
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
