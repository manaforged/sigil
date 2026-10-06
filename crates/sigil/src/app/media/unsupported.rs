use super::types::{MediaKey, Track};
use crate::app::error::{AppResult, Error};

pub(super) type Controls = ();

pub(super) fn supported() -> AppResult<()> {
    Err(unsupported())
}

pub(super) fn create(
    _name: &str,
    _hwnd: Option<isize>,
    _sink: impl Fn(MediaKey) + Send + Sync + 'static,
) -> AppResult<Controls> {
    Err(unsupported())
}

pub(super) fn apply(_controls: &mut Controls, _track: &Track) -> AppResult<()> {
    Err(unsupported())
}

fn unsupported() -> Error {
    Error::new(std::io::Error::from(std::io::ErrorKind::Unsupported))
}
